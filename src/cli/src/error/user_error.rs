use crate::CliError;
use fluvio::FluvioError;

#[derive(thiserror::Error, Debug)]
pub enum UserError {

}

impl From<CliError> for UserError {
    fn from(error: CliError) -> Self {
        match error {
            _ => unimplemented!()
        }

        todo!()
    }
}
