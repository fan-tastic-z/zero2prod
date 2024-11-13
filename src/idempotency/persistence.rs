use std::str::FromStr;

use axum::{
    body::{to_bytes, Body},
    http::{HeaderName, HeaderValue},
    response::Response,
};
use bytes::Bytes;
use hyper::HeaderMap;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;
use crate::{errors::Error, Result};

#[derive(Debug, Clone, sqlx::Type, sqlx::FromRow)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

#[derive(Debug, sqlx::FromRow)]
struct IdempotencyRecord {
    response_status_code: i16,
    response_headers: Vec<HeaderPairRecord>,
    response_body: Vec<u8>,
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<Response<Body>>> {
    let saved: Option<IdempotencyRecord> = sqlx::query_as(
        r#"
        SELECT
            response_status_code,
            response_headers,
            response_body
        FROM idempotency
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
    )
    .bind(user_id)
    .bind(idempotency_key.as_ref())
    .fetch_optional(pool)
    .await?;
    if let Some(r) = saved {
        let mut header_map = HeaderMap::new();

        for header in r.response_headers {
            let header_name = HeaderName::from_str(&header.name).expect("Invalid header name");
            let header_value =
                HeaderValue::from_bytes(&header.value).expect("Invalid header value");
            header_map.insert(header_name, header_value);
        }
        let mut response = Response::builder().status(r.response_status_code as u16);
        for (header_name, header_value) in header_map.iter() {
            response = response.header(header_name, header_value);
        }
        let body = Bytes::from(r.response_body);
        let response = response.body(Body::from(body))?;
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    http_response: Response<Body>,
) -> Result<Response<Body>> {
    let status_code = http_response.status().as_u16() as i16;
    let (response_head, body) = http_response.into_parts();
    let body = to_bytes(body, usize::MAX).await?;
    let headers = {
        let mut h = Vec::with_capacity(response_head.headers.len());
        for (name, value) in response_head.headers.iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    let query = sqlx::query(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
    )
    .bind(user_id)
    .bind(idempotency_key.as_ref())
    .bind(status_code)
    .bind(headers.clone())
    .bind(body.as_ref());

    transaction.execute(query).await?;
    transaction.commit().await?;

    let mut response = Response::builder().status(status_code as u16);
    for header in headers.into_iter() {
        response = response.header(header.name, header.value);
    }
    let response = response.body(Body::from(body))?;
    Ok(response)
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response<Body>),
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction> {
    let mut transaction = pool.begin().await?;
    let query = sqlx::query(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(idempotency_key.as_ref());
    let n_inserted_rows = transaction.execute(query).await?.rows_affected();
    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| Error::InvalidIdempotencyKey)?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
