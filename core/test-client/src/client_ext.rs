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

//! Client extension for tests.

use client::{self, Client};
use consensus::{BlockImport, BlockOrigin, ImportBlock};
use primitives::Blake2Hasher;
use runtime;
use runtime_primitives::generic::BlockId;
use runtime_primitives::Justification;

/// Extension trait for a test client.
pub trait TestClient: Sized {
    /// Import block to the chain. No finality.
    fn import(&self, origin: BlockOrigin, block: runtime::Block) -> client::error::Result<()>;

    /// Import block with justification, finalizes block.
    fn import_justified(
        &self,
        origin: BlockOrigin,
        block: runtime::Block,
        justification: Justification,
    ) -> client::error::Result<()>;

    /// Finalize a block.
    fn finalize_block(
        &self,
        id: BlockId<runtime::Block>,
        justification: Option<Justification>,
    ) -> client::error::Result<()>;

    /// Returns hash of the genesis block.
    fn genesis_hash(&self) -> runtime::Hash;
}

impl<B, E, RA> TestClient for Client<B, E, runtime::Block, RA>
where
    B: client::backend::Backend<runtime::Block, Blake2Hasher>,
    E: client::CallExecutor<runtime::Block, Blake2Hasher>,
    Self: BlockImport<runtime::Block, Error = client::error::Error>,
{
    fn import(&self, origin: BlockOrigin, block: runtime::Block) -> client::error::Result<()> {
        let import = ImportBlock {
            origin,
            header: block.header,
            justification: None,
            post_digests: vec![],
            body: Some(block.extrinsics),
            finalized: false,
            auxiliary: Vec::new(),
        };

        self.import_block(import, None).map(|_| ())
    }

    fn import_justified(
        &self,
        origin: BlockOrigin,
        block: runtime::Block,
        justification: Justification,
    ) -> client::error::Result<()> {
        let import = ImportBlock {
            origin,
            header: block.header,
            justification: Some(justification),
            post_digests: vec![],
            body: Some(block.extrinsics),
            finalized: true,
            auxiliary: Vec::new(),
        };

        self.import_block(import, None).map(|_| ())
    }

    fn finalize_block(
        &self,
        id: BlockId<runtime::Block>,
        justification: Option<Justification>,
    ) -> client::error::Result<()> {
        self.finalize_block(id, justification, true)
    }

    fn genesis_hash(&self) -> runtime::Hash {
        self.block_hash(0).unwrap().unwrap()
    }
}
