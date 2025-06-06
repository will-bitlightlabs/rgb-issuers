// RGB issuers
//
// SPDX-License-Identifier: Apache-2.0
//
// Designed in 2019-2025 by Dr Maxim Orlovsky <orlovsky@pandoraprime.ch>
// Written in 2024-2025 by Dr Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2019-2022 Pandora Core SA, Neuchatel, Switzerland.
// Copyright (C) 2022-2025 Pandora Prime Inc, Neuchatel, Switzerland.
// Copyright (C) 2019-2025 Dr Maxim Orlovsky.
// All rights under the above copyrights are reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License. You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

#[macro_use]
extern crate amplify;

use hypersonic::{Codex, Identity, Schema};
use ifaces::Rgb21Types;
use issuers::scripts::{self, shared_lib, unique, FN_FAC_TRANSFER, FN_RGB21_ISSUE};
use issuers::PANDORA;
use std::fs;
use zkaluvm::alu::{CoreConfig, Lib};
use zkaluvm::FIELD_ORDER_SECP;

fn codex() -> (Codex, Lib) {
    let lib = scripts::fractionable();
    let codex = Codex {
        name: tiny_s!("Fractional asset collection"),
        developer: Identity::from(PANDORA),
        version: default!(),
        timestamp: 1732529307,
        field_order: FIELD_ORDER_SECP,
        input_config: CoreConfig::default(),
        verification_config: CoreConfig::default(),
        verifiers: tiny_bmap! {
            0 => lib.routine(FN_RGB21_ISSUE),
            1 => lib.routine(FN_FAC_TRANSFER),
            0xFF => lib.routine(FN_FAC_TRANSFER), // Blank transition is just an ordinary self-transfer
        },
        reserved: default!(),
    };
    (codex, lib.into_lib())
}

fn main() {
    const FILE: &str = "compiled/RGB21-FAC.issuer";

    let types = Rgb21Types::new();
    let (codex, lib) = codex();
    let api = issuers::ifaces::rgb21::api(codex.codex_id());

    // Creating DAO with three participants
    let issuer = Schema::new(
        codex,
        api,
        [shared_lib().into_lib(), unique().into_lib(), lib],
        types.type_system(),
    );
    println!(
        "Created issuer '{}' with id {}",
        issuer.codex.name,
        issuer.codex.codex_id()
    );
    let _ = fs::remove_file(FILE);
    issuer
        .save(FILE)
        .expect("unable to save the issuer to the file");
}
