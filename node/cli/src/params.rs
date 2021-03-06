// Copyright 2018 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use cli::CoreParams;
use structopt::StructOpt;

/// Extend params for Node
#[derive(Debug, StructOpt)]
pub struct Params {
    /// Should run as a GRANDPA authority node
    #[structopt(
        long = "grandpa-authority",
        help = "Run Node as a GRANDPA authority, implies --validator"
    )]
    grandpa_authority: bool,

    /// Should run as a GRANDPA authority node only
    #[structopt(
        long = "grandpa-authority-only",
        help = "Run Node as a GRANDPA authority only, don't as a usual validator, implies --grandpa-authority"
    )]
    grandpa_authority_only: bool,

    #[structopt(flatten)]
    core: CoreParams,
}
