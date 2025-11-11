use axum::{extract::State, Json, Extension};
use axum::http::StatusCode;
use crate::state::AppState;
use crate::error::AppError;
use crate::dtos::sale::{
    CreateSaleRequest, UpdatePaymentRequest, SaleResponse, 
    SaleItemResponse, SaleSummary, SaleListItem
};
use crate::middleware::auth::AuthContext;
use sqlx::PgPool;

pub async fn create_sale(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateSaleRequest>,
) -> Result<(StatusCode, Json<SaleResponse>), AppError> {
    if req.items.is_empty() {
        return Err(AppError::validation("Sale must contain at least one item"));
    }

    // Start transaction
    let mut tx = db_pool.begin().await?;

    // Verify truck load exists and get truck info
    let truck_load = sqlx::query!(
        r#"SELECT tl.id, tl.truck_id, t.truck_number, t.driver_id, u.username as driver_username
        FROM truck_loads tl
        JOIN trucks t ON tl.truck_id = t.id
        JOIN users u ON t.driver_id = u.id
        WHERE tl.id = $1"#,
        req.truck_load_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Truck load not found"))?;

    // Verify driver can only create sales for their own truck
    if auth.role == "driver" && truck_load.driver_id != Some(auth.user_id) {
        return Err(AppError::forbidden("You can only create sales for your own truck"));
    }

    // Verify shop exists
    let shop = sqlx::query!(
        r#"SELECT id, name FROM shops WHERE id = $1"#,
        req.shop_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Shop not found"))?;

    // Calculate total amount and prepare items
    let mut total_amount: f64 = 0.0;
    let mut sale_items = Vec::new();

    for item in &req.items {
        if item.quantity <= 0 {
            return Err(AppError::validation("Quantity must be greater than 0"));
        }

        // Get product info
        let product = sqlx::query!(
            r#"SELECT id, name, (current_wholesale_price)::FLOAT8 as "current_wholesale_price!", 
               (commission_per_unit)::FLOAT8 as "commission_per_unit!"
            FROM products WHERE id = $1"#,
            item.product_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found(&format!("Product {} not found", item.product_id)))?;

        // Use provided unit_price or default to current_wholesale_price
        let unit_price = item.unit_price.unwrap_or(product.current_wholesale_price);

        if unit_price < 0.0 {
            return Err(AppError::validation("Unit price cannot be negative"));
        }

        // Find available batch from truck load (FIFO by expiry_date)
        let batch = sqlx::query!(
            r#"SELECT 
                tli.batch_id,
                b.batch_number,
                b.expiry_date,
                tli.quantity_loaded,
                tli.quantity_sold,
                tli.quantity_returned
            FROM truck_load_items tli
            JOIN batches b ON tli.batch_id = b.id
            WHERE tli.truck_load_id = $1 
            AND b.product_id = $2
            AND (tli.quantity_loaded - tli.quantity_sold - tli.quantity_returned) >= $3
            ORDER BY b.expiry_date ASC, b.created_at ASC
            LIMIT 1"#,
            req.truck_load_id,
            item.product_id,
            item.quantity
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::validation(&format!(
            "Insufficient quantity for product '{}' in truck load. Need {}, but not enough available.",
            product.name, item.quantity
        )))?;

        // Calculate commission (always fixed per unit)
        let commission_earned = item.quantity as f64 * product.commission_per_unit;
        let line_total = item.quantity as f64 * unit_price;

        total_amount += line_total;

        sale_items.push((
            item.product_id,
            product.name.clone(),
            batch.batch_id,
            batch.batch_number.clone(),
            item.quantity,
            unit_price,
            commission_earned,
            line_total,
        ));
    }

    // Set amount_paid (default to 0 if not provided)
    let amount_paid = req.amount_paid.unwrap_or(0.0);

    if amount_paid < 0.0 {
        return Err(AppError::validation("Amount paid cannot be negative"));
    }

    if amount_paid > total_amount {
        return Err(AppError::validation("Amount paid cannot exceed total amount"));
    }

    // Determine payment status
    let payment_status = if amount_paid >= total_amount {
        "paid"
    } else {
        "pending"
    };

    // Create sale record
    let sale = sqlx::query!(
        r#"INSERT INTO sales (shop_id, truck_id, user_id, truck_load_id, total_amount, amount_paid, payment_status, sale_date)
        VALUES ($1, $2, $3, $4, $5::FLOAT8, $6::FLOAT8, $7, $8)
        RETURNING id, shop_id, truck_id, user_id, truck_load_id, (total_amount)::FLOAT8 as "total_amount!", 
                  (amount_paid)::FLOAT8 as "amount_paid!", payment_status, sale_date, created_at"#,
        req.shop_id,
        truck_load.truck_id,
        auth.user_id,
        req.truck_load_id,
        total_amount,
        amount_paid,
        payment_status,
        req.sale_date
    )
    .fetch_one(&mut *tx)
    .await?;

    // Insert sale items and collect response data
    let mut item_responses = Vec::new();
    let mut total_commission = 0.0;

    for (product_id, product_name, batch_id, batch_number, quantity, unit_price, commission, line_total) in sale_items {
        let sale_item = sqlx::query!(
            r#"INSERT INTO sale_items (sale_id, batch_id, quantity, unit_price, commission_earned)
            VALUES ($1, $2, $3, $4::FLOAT8, $5::FLOAT8)
            RETURNING id"#,
            sale.id,
            batch_id,
            quantity,
            unit_price,
            commission
        )
        .fetch_one(&mut *tx)
        .await?;

        total_commission += commission;

        item_responses.push(SaleItemResponse {
            id: sale_item.id,
            product_id,
            product_name,
            batch_id,
            batch_number,
            quantity,
            unit_price,
            commission_earned: commission,
            line_total,
        });
    }

    // Commit transaction
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(SaleResponse {
            id: sale.id,
            shop_id: sale.shop_id,
            shop_name: shop.name,
            truck_id: sale.truck_id,
            truck_number: truck_load.truck_number,
            driver_id: truck_load.driver_id.unwrap(),
            driver_username: truck_load.driver_username,
            truck_load_id: sale.truck_load_id.unwrap(),
            total_amount: sale.total_amount,
            amount_paid: sale.amount_paid,
            payment_status: sale.payment_status,
            sale_date: sale.sale_date,
            created_at: sale.created_at.unwrap(),
            items: item_responses,
            summary: SaleSummary {
                total_items: req.items.iter().map(|i| i.quantity).sum(),
                total_commission,
                balance_due: sale.total_amount - sale.amount_paid,
            },
        }),
    ))
}

pub async fn get_sale(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<SaleResponse>, AppError> {
    fetch_sale_by_id(&db_pool, id).await.map(Json)
}

pub async fn list_sales(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<SaleListItem>>, AppError> {
    let driver_id = params.get("driver_id").and_then(|s| s.parse::<i64>().ok());
    let shop_id = params.get("shop_id").and_then(|s| s.parse::<i64>().ok());
    let sale_date = params.get("sale_date").and_then(|s| s.parse::<chrono::NaiveDate>().ok());
    let payment_status = params.get("payment_status");

    let mut query_str = String::from(
        r#"SELECT 
            s.id, s.sale_date, s.payment_status,
            (s.total_amount)::FLOAT8 as total_amount,
            (s.amount_paid)::FLOAT8 as amount_paid,
            sh.name as shop_name,
            t.truck_number,
            u.username as driver_username,
            COUNT(si.id)::INT as total_items
        FROM sales s
        JOIN shops sh ON s.shop_id = sh.id
        JOIN trucks t ON s.truck_id = t.id
        JOIN users u ON s.user_id = u.id
        LEFT JOIN sale_items si ON s.id = si.sale_id
        WHERE 1=1"#
    );

    if driver_id.is_some() {
        query_str.push_str(" AND s.user_id = $1");
    }
    if shop_id.is_some() {
        let param_num = if driver_id.is_some() { 2 } else { 1 };
        query_str.push_str(&format!(" AND s.shop_id = ${}", param_num));
    }
    if sale_date.is_some() {
        let param_num = if driver_id.is_some() && shop_id.is_some() { 3 }
                       else if driver_id.is_some() || shop_id.is_some() { 2 }
                       else { 1 };
        query_str.push_str(&format!(" AND s.sale_date = ${}", param_num));
    }
    if payment_status.is_some() {
        let param_num = if driver_id.is_some() && shop_id.is_some() && sale_date.is_some() { 4 }
                       else if (driver_id.is_some() as u8 + shop_id.is_some() as u8 + sale_date.is_some() as u8) == 2 { 3 }
                       else if driver_id.is_some() || shop_id.is_some() || sale_date.is_some() { 2 }
                       else { 1 };
        query_str.push_str(&format!(" AND s.payment_status = ${}", param_num));
    }

    query_str.push_str(" GROUP BY s.id, s.sale_date, s.payment_status, s.total_amount, s.amount_paid, sh.name, t.truck_number, u.username ORDER BY s.sale_date DESC, s.id DESC");

    let mut query = sqlx::query_as::<_, (i64, chrono::NaiveDate, String, f64, f64, String, String, String, i32)>(&query_str);

    if let Some(did) = driver_id {
        query = query.bind(did);
    }
    if let Some(sid) = shop_id {
        query = query.bind(sid);
    }
    if let Some(date) = sale_date {
        query = query.bind(date);
    }
    if let Some(status) = payment_status {
        query = query.bind(status);
    }

    let sales = query.fetch_all(&db_pool).await?;

    Ok(Json(
        sales
            .into_iter()
            .map(|(id, sale_date, payment_status, total_amount, amount_paid, shop_name, truck_number, driver_username, total_items)| {
                SaleListItem {
                    id,
                    shop_name,
                    truck_number,
                    driver_username,
                    total_amount,
                    amount_paid,
                    payment_status,
                    sale_date,
                    total_items,
                }
            })
            .collect(),
    ))
}

pub async fn update_payment(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<UpdatePaymentRequest>,
) -> Result<Json<SaleResponse>, AppError> {
    if req.additional_payment <= 0.0 {
        return Err(AppError::validation("Additional payment must be greater than 0"));
    }

    // Start transaction
    let mut tx = db_pool.begin().await?;

    // Get sale and verify ownership if driver
    let sale = sqlx::query!(
        r#"SELECT s.id, s.user_id, s.truck_id, (s.total_amount)::FLOAT8 as "total_amount!", 
           (s.amount_paid)::FLOAT8 as "amount_paid!"
        FROM sales s
        WHERE s.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Sale not found"))?;

    // If driver, verify they own this sale
    if auth.role == "driver" && sale.user_id != auth.user_id {
        return Err(AppError::forbidden("You can only update payments for your own sales"));
    }

    let new_amount_paid = sale.amount_paid + req.additional_payment;

    if new_amount_paid > sale.total_amount {
        return Err(AppError::validation(&format!(
            "Total payment ({}) would exceed sale amount ({})",
            new_amount_paid, sale.total_amount
        )));
    }

    // Update payment
    let payment_status = if new_amount_paid >= sale.total_amount {
        "paid"
    } else {
        "pending"
    };

    sqlx::query!(
        r#"UPDATE sales 
        SET amount_paid = $2::FLOAT8, payment_status = $3
        WHERE id = $1"#,
        id,
        new_amount_paid,
        payment_status
    )
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    // Fetch and return updated sale
    fetch_sale_by_id(&db_pool, id).await.map(Json)
}

// Helper function to fetch full sale details
async fn fetch_sale_by_id(db_pool: &PgPool, id: i64) -> Result<SaleResponse, AppError> {
    // Fetch sale header
    let sale = sqlx::query!(
        r#"SELECT 
            s.id, s.shop_id, s.truck_id, s.user_id, s.truck_load_id, s.sale_date,
            (s.total_amount)::FLOAT8 as "total_amount!",
            (s.amount_paid)::FLOAT8 as "amount_paid!",
            s.payment_status, s.created_at,
            sh.name as shop_name,
            t.truck_number,
            u.username as driver_username
        FROM sales s
        JOIN shops sh ON s.shop_id = sh.id
        JOIN trucks t ON s.truck_id = t.id
        JOIN users u ON s.user_id = u.id
        WHERE s.id = $1"#,
        id
    )
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Sale not found"))?;

    // Fetch sale items
    let items_data = sqlx::query!(
        r#"SELECT 
            si.id, si.batch_id, si.quantity,
            (si.unit_price)::FLOAT8 as "unit_price!",
            (si.commission_earned)::FLOAT8 as "commission_earned!",
            b.batch_number, b.product_id,
            p.name as product_name
        FROM sale_items si
        JOIN batches b ON si.batch_id = b.id
        JOIN products p ON b.product_id = p.id
        WHERE si.sale_id = $1
        ORDER BY si.id"#,
        id
    )
    .fetch_all(db_pool)
    .await?;

    let mut total_items = 0;
    let mut total_commission = 0.0;

    let items: Vec<SaleItemResponse> = items_data
        .into_iter()
        .map(|item| {
            total_items += item.quantity;
            total_commission += item.commission_earned;
            let line_total = item.quantity as f64 * item.unit_price;

            SaleItemResponse {
                id: item.id,
                product_id: item.product_id,
                product_name: item.product_name,
                batch_id: item.batch_id,
                batch_number: item.batch_number,
                quantity: item.quantity,
                unit_price: item.unit_price,
                commission_earned: item.commission_earned,
                line_total,
            }
        })
        .collect();

    Ok(SaleResponse {
        id: sale.id,
        shop_id: sale.shop_id,
        shop_name: sale.shop_name,
        truck_id: sale.truck_id,
        truck_number: sale.truck_number,
        driver_id: sale.user_id,
        driver_username: sale.driver_username,
        truck_load_id: sale.truck_load_id.unwrap(),
        total_amount: sale.total_amount,
        amount_paid: sale.amount_paid,
        payment_status: sale.payment_status,
        sale_date: sale.sale_date,
        created_at: sale.created_at.unwrap(),
        items,
        summary: SaleSummary {
            total_items,
            total_commission,
            balance_due: sale.total_amount - sale.amount_paid,
        },
    })
}
