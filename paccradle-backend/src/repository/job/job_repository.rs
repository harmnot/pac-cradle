use std::time::Duration;

use sqlx::{PgConnection, PgPool};

use super::JobRepository;
use crate::error::AppError;
use crate::models::job::{Job, NewJob};
use crate::shared::future::BoxFuture;

/// Written to `last_error` when a lease runs out.
const LEASE_EXPIRED: &str = "the worker stopped before the job finished (its lease ran out)";

/// [`JobRepository`] on Postgres.
pub struct PgJobRepository {
    pool: PgPool,
}

impl PgJobRepository {
    /// A repository using `pool`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl JobRepository for PgJobRepository {
    fn insert<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        job: &'a NewJob,
    ) -> BoxFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query!(
                r#"
                INSERT INTO jobs (id, kind, payload, max_attempts, run_at)
                VALUES ($1, $2, $3, $4, now() + make_interval(secs => $5))
                "#,
                job.id,
                job.kind,
                job.payload,
                job.max_attempts,
                job.delay.as_secs_f64(),
            )
            .execute(conn)
            .await?;
            Ok(())
        })
    }

    fn claim<'a>(
        &'a self,
        kinds: &'a [&'static str],
        lease: Duration,
    ) -> BoxFuture<'a, Result<Option<Job>, AppError>> {
        Box::pin(async move {
            let job = sqlx::query_as!(
                Job,
                r#"
                UPDATE jobs
                SET status = 'running',
                    attempts = attempts + 1,
                    locked_until = now() + make_interval(secs => $2)
                WHERE id = (
                    SELECT id FROM jobs
                    WHERE status = 'queued' AND run_at <= now() AND kind = ANY($1)
                    ORDER BY run_at, id
                    LIMIT 1
                    FOR UPDATE SKIP LOCKED
                )
                RETURNING id, kind, payload, attempts, max_attempts
                "#,
                kinds as &[&str],
                lease.as_secs_f64(),
            )
            .fetch_optional(&self.pool)
            .await?;
            Ok(job)
        })
    }

    fn mark_done<'a>(&'a self, job: &'a Job) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE jobs
                SET status = 'done', locked_until = NULL, finished_at = now(), last_error = NULL
                WHERE id = $1 AND status = 'running' AND attempts = $2
                "#,
                job.id,
                job.attempts,
            )
            .execute(&self.pool)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn requeue<'a>(
        &'a self,
        job: &'a Job,
        (delay, error): (Duration, &'a str),
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE jobs
                SET status = 'queued',
                    locked_until = NULL,
                    run_at = now() + make_interval(secs => $3),
                    last_error = $4
                WHERE id = $1 AND status = 'running' AND attempts = $2
                "#,
                job.id,
                job.attempts,
                delay.as_secs_f64(),
                error,
            )
            .execute(&self.pool)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn mark_dead<'a>(
        &'a self,
        job: &'a Job,
        error: &'a str,
    ) -> BoxFuture<'a, Result<bool, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                UPDATE jobs
                SET status = 'dead', locked_until = NULL, finished_at = now(), last_error = $3
                WHERE id = $1 AND status = 'running' AND attempts = $2
                "#,
                job.id,
                job.attempts,
                error,
            )
            .execute(&self.pool)
            .await?;
            Ok(result.rows_affected() == 1)
        })
    }

    fn release_expired(&self) -> BoxFuture<'_, Result<(u64, u64), AppError>> {
        Box::pin(async move {
            let counts = sqlx::query!(
                r#"
                WITH released AS (
                    UPDATE jobs
                    SET status = CASE WHEN attempts >= max_attempts THEN 'dead' ELSE 'queued' END,
                        finished_at = CASE WHEN attempts >= max_attempts THEN now() END,
                        run_at = now(),
                        locked_until = NULL,
                        last_error = $1
                    WHERE status = 'running' AND locked_until < now()
                    RETURNING status
                )
                SELECT count(*) FILTER (WHERE status = 'queued') AS "requeued!",
                       count(*) FILTER (WHERE status = 'dead') AS "dead!"
                FROM released
                "#,
                LEASE_EXPIRED,
            )
            .fetch_one(&self.pool)
            .await?;
            Ok((count(counts.requeued), count(counts.dead)))
        })
    }

    fn delete_done_older_than(&self, age: Duration) -> BoxFuture<'_, Result<u64, AppError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                r#"
                DELETE FROM jobs
                WHERE status = 'done' AND finished_at < now() - make_interval(secs => $1)
                "#,
                age.as_secs_f64(),
            )
            .execute(&self.pool)
            .await?;
            Ok(result.rows_affected())
        })
    }
}

/// A SQL count, which is never negative.
fn count(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}
