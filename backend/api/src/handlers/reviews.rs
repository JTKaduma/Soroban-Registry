use ax_auth::AuthUser;
use axum::{extract::{Path, State, Query}, Json, response::IntoResponse, http::StatusCode};
use crate::models::{CreateReviewRequest, ReviewResponse, ContractRatingStats};
use sqlx::PgPool;
use uuid::Uuid;

/// POST /api/contracts/:id/reviews
pub async fn create_review(
    State(pool): State<PgPool>,
    Path(contract_id): Path<Uuid>,
    Json(payload): Json<CreateReviewRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    //  Validation logic
    if payload.rating < 1.0 || payload.rating > 5.0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Insert Review
    let review = sqlx::query_as!(
        ReviewResponse,
        r#"
        INSERT INTO reviews (contract_id, user_id, version, rating, review_text)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, contract_id, user_id, version, rating::float4 as "rating!", review_text, helpful_count, is_flagged, created_at
        "#,
        contract_id,
        Uuid::new_v4(),
        payload.version,
        payload.rating as f64,
        payload.review_text
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(review)))
}

///Endpoint: GET /api/contracts/:id/reviews
pub async fn get_reviews(
    State(pool): State<PgPool>,
    Path(contract_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let reviews = sqlx::query_as!(
        ReviewResponse,
        r#"
        SELECT id, contract_id, user_id, version, rating::float4 as "rating!", review_text, helpful_count, is_flagged, created_at
        FROM reviews
        WHERE contract_id = $1 AND is_flagged = FALSE
        ORDER BY created_at DESC
        "#,
        contract_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(reviews))
}


pub async fn get_contract_stats(pool: &PgPool, contract_id: Uuid) -> ContractRatingStats {
    let row = sqlx::query!(
        r#"
        SELECT AVG(rating)::float8 as avg_rating, COUNT(*) as count
        FROM reviews
        WHERE contract_id = $1 AND is_flagged = FALSE
        "#,
        contract_id
    )
    .fetch_one(pool)
    .await
    .unwrap_or_default();

    ContractRatingStats {
        average_rating: row.avg_rating.unwrap_or(0.0),
        total_reviews: row.count.unwrap_or(0),
    }
}