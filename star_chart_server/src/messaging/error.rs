use thiserror::Error;

use crate::database::AppDatabaseError;

#[derive(Error, Debug)]
pub enum ServerError {

    #[error(transparent)]
    Database(#[from] AppDatabaseError)
}
