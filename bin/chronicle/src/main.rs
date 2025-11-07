// This file is part of Chronicle.
//
// Chronicle is free software: you can redistribute it and/or modify it under the
// terms of the GNU Lesser General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later version.
//
// Chronicle is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Chronicle.
// If not, see https://www.gnu.org/licenses/.

pub mod cli;

use tracing::{error, info};
use crate::cli::utils::{handle_user_friendly_error};

#[tokio::main]
async fn main() {


    match cli::run().await {

        Ok(()) => {
            info!("Chronicle completed successfully");
            std::process::exit(0);
        }
        Err(err) => {
            error!("Application error: {err:?}");

            handle_user_friendly_error(&err);

            std::process::exit(1);
        }
    }
}
