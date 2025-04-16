use crate::errors::AppError;
use tracing::log::{debug, error};

pub(super) async fn run_query<T, F>(
    pool: &deadpool_diesel::postgres::Pool,
    query: F,
) -> Result<T, AppError>
where
    F: FnOnce(&mut diesel::PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
    T: Send + 'static,
{
    let conn = pool.get().await.map_err(|pool_err| {
        error!(
            "Failed to get DB connection object from pool: {:?}",
            pool_err
        );
        AppError::PoolError(pool_err)
    })?;
    debug!("DB connection object obtained from pool for interaction");

    let res = conn.interact(query).await;

    match res {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(diesel_err)) => {
            error!("Diesel query failed within interaction: {:?}", diesel_err);
            Err(AppError::DieselError(diesel_err))
        }
        Err(interact_err) => {
            error!("Deadpool interact error: {:?}", interact_err);
            Err(AppError::InteractError(interact_err))
        }
    }
}
