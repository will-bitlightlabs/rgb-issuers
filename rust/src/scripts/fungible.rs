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

use super::{shared_lib, FN_ASSET_SPEC, FN_SUM_INPUTS, FN_SUM_OUTPUTS};
use crate::G_SUPPLY;
use hypersonic::uasm;
use zkaluvm::alu::CompiledLib;

pub const FN_FUNGIBLE_ISSUE: u16 = 0;
pub const FN_FUNGIBLE_TRANSFER: u16 = 1;

pub fn fungible() -> CompiledLib {
    let shared = shared_lib().into_lib().lib_id();

    let mut code = uasm! {
     .routine: FN_FUNGIBLE_ISSUE;
        call    shared, :FN_ASSET_SPEC   ;// Call asset check
        fits    EB, 8:bits      ;// The precision must fit into a byte
        chk     CO              ;// - or fail otherwise

        // Validate circulating supply
        ldo     :immutable      ;// Read last global state - circulating supply
        chk     CO              ;// It must exist
        mov     E8, :G_SUPPLY   ;// Load supply type
        eq      EA, E8          ;// It must have a correct state type
        chk     CO              ;// Or fail otherwise
        test    EB              ;// It must be set
        chk     CO              ;// Or we should fail
        mov     E2, EB          ;// Save supply
        test    EC              ;// ensure other field elements are empty
        not     CO              ;// invert CO value (we need the test to fail)
        chk     CO              ;// fail if not
        test    ED              ;// ensure other field elements are empty
        not     CO              ;// invert CO value (we need the test to fail)
        chk     CO              ;// fail if not
        mov     E3, 0           ;// E3 will contain the sum of outputs
        clr     EE              ;// Ensure EE is set to none, so we enforce the third element to be empty
        call    shared, :FN_SUM_OUTPUTS    ;// Compute a sum of outputs
        eq      E2, E3          ;// check that circulating supply equals to the sum of outputs
        chk     CO              ;// fail if not

        // Check there is no more global state
        ldo     :immutable      ;
        not     CO              ;
        chk     CO              ;
        ret;

     .routine: FN_FUNGIBLE_TRANSFER;
        // Verify that no global state is defined
        ldo     :immutable      ;// Try to iterate over global state
        not     CO              ;// Invert result (we need NO state as a Success)
        chk     CO              ;// Fail if there is a global state

        // Verify owned state
        clr     EE              ;// Ensure EE is set to none, so we enforce the third element to be empty
        call    shared, :FN_SUM_INPUTS     ;// Compute a sum of inputs into E2
        call    shared, :FN_SUM_OUTPUTS    ;// Compute a sum of outputs into E3
        eq      E2, E3          ;// check that the sum of inputs equals the sum of outputs
        chk     CO              ;// fail if not

        ret;
    };

    CompiledLib::compile(&mut code, &[&shared_lib()])
        .unwrap_or_else(|err| panic!("Invalid script: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{G_NAME, G_PRECISION, G_SUPPLY, G_TICKER, O_AMOUNT};
    use hypersonic::{AuthToken, Instr, StateCell, StateData, StateValue, VmContext};
    use strict_types::StrictDumb;
    use zkaluvm::alu::{CoreConfig, Lib, LibId, Vm};
    use zkaluvm::{GfaConfig, FIELD_ORDER_SECP};

    const CONFIG: CoreConfig = CoreConfig {
        halt: true,
        complexity_lim: Some(500_000_000),
    };

    fn harness() -> (CompiledLib, Vm<Instr<LibId>>, impl Fn(LibId) -> Option<Lib>) {
        let vm = Vm::<Instr<LibId>>::with(
            CONFIG,
            GfaConfig {
                field_order: FIELD_ORDER_SECP,
            },
        );
        fn resolver(id: LibId) -> Option<Lib> {
            let fungible = fungible();
            let shared = shared_lib();
            if fungible.as_lib().lib_id() == id {
                return Some(fungible.into_lib());
            }
            if shared.as_lib().lib_id() == id {
                return Some(shared.into_lib());
            }
            panic!("Unknown library: {id}");
        }
        (fungible(), vm, resolver)
    }

    #[test]
    fn genesis_empty() {
        let context = VmContext {
            read_once_input: &[],
            immutable_input: &[],
            read_once_output: &[],
            immutable_output: &[],
        };
        let (lib, mut vm, resolver) = harness();
        let res = vm
            .exec(lib.routine(FN_FUNGIBLE_ISSUE), &context, resolver)
            .is_ok();
        assert!(!res);
    }

    #[test]
    fn genesis_missing_globals() {
        let mut context = VmContext {
            read_once_input: &[],
            immutable_input: &[],
            read_once_output: &[StateCell {
                data: StateValue::new(O_AMOUNT, 1000_u64),
                auth: AuthToken::strict_dumb(),
                lock: None,
            }],
            immutable_output: &[],
        };
        let globals = [
            &[
                StateData::new(G_TICKER, 0u8),
                StateData::new(G_PRECISION, 18_u8),
                StateData::new(G_SUPPLY, 1000_u64),
            ][..],
            &[
                StateData::new(G_NAME, 0u8),
                StateData::new(G_PRECISION, 18_u8),
                StateData::new(G_SUPPLY, 1000_u64),
            ],
            &[
                StateData::new(G_NAME, 0u8),
                StateData::new(G_TICKER, 0u8),
                StateData::new(G_SUPPLY, 1000_u64),
            ],
            &[
                StateData::new(G_NAME, 0u8),
                StateData::new(G_TICKER, 0u8),
                StateData::new(G_PRECISION, 18_u8),
            ],
            &[StateData::new(G_NAME, 0u8), StateData::new(G_TICKER, 0u8)],
        ];
        for global in globals {
            context.immutable_output = global;
            let (lib, mut vm, resolver) = harness();
            let res = vm
                .exec(lib.routine(FN_FUNGIBLE_ISSUE), &context, resolver)
                .is_ok();
            assert!(!res);
        }
    }

    #[test]
    fn genesis_missing_owned() {
        let context = VmContext {
            read_once_input: &[],
            immutable_input: &[],
            read_once_output: &[],
            immutable_output: &[
                StateData::new(G_NAME, 0u8),
                StateData::new(G_TICKER, 0u8),
                StateData::new(G_PRECISION, 18_u8),
                StateData::new(G_SUPPLY, 1000_u64),
            ],
        };
        let (lib, mut vm, resolver) = harness();
        let res = vm
            .exec(lib.routine(FN_FUNGIBLE_ISSUE), &context, resolver)
            .is_ok();
        assert!(!res);
    }

    #[test]
    fn genesis_supply_mismatch() {
        let context = VmContext {
            read_once_input: &[],
            immutable_input: &[],
            read_once_output: &[StateCell {
                data: StateValue::new(O_AMOUNT, 1001_u64),
                auth: AuthToken::strict_dumb(),
                lock: None,
            }],
            immutable_output: &[
                StateData::new(G_NAME, 0u8),
                StateData::new(G_TICKER, 0u8),
                StateData::new(G_PRECISION, 18_u8),
                StateData::new(G_SUPPLY, 1000_u64),
            ],
        };
        let (lib, mut vm, resolver) = harness();
        let res = vm
            .exec(lib.routine(FN_FUNGIBLE_ISSUE), &context, resolver)
            .is_ok();
        assert!(!res);
    }

    #[test]
    fn genesis_correct() {
        let context = VmContext {
            read_once_input: &[],
            immutable_input: &[],
            read_once_output: &[StateCell {
                data: StateValue::new(O_AMOUNT, 1000_u64),
                auth: AuthToken::strict_dumb(),
                lock: None,
            }],
            immutable_output: &[
                StateData::new(G_TICKER, 0u8),
                StateData::new(G_NAME, 0u8),
                StateData::new(G_PRECISION, 18_u8),
                StateData::new(G_SUPPLY, 1000_u64),
            ],
        };
        let (lib, mut vm, resolver) = harness();
        let res = vm
            .exec(lib.routine(FN_FUNGIBLE_ISSUE), &context, resolver)
            .is_ok();
        assert!(res);
    }

    fn transfer_harness(inp: &[&[u64]], out: &[&[u64]], should_success: bool) {
        let inputs = inp.into_iter().map(|vals| {
            vals.into_iter()
                .map(|val| StateValue::new(O_AMOUNT, *val))
                .collect::<Vec<_>>()
        });
        let lock = None;
        let auth = AuthToken::strict_dumb();
        let outputs = out.into_iter().map(|vals| {
            vals.into_iter()
                .map(|val| StateCell {
                    data: StateValue::new(O_AMOUNT, *val),
                    auth,
                    lock,
                })
                .collect::<Vec<_>>()
        });
        for (input, output) in inputs.flat_map(|inp| {
            outputs
                .clone()
                .into_iter()
                .map(move |out| (inp.clone(), out))
        }) {
            let (lib, mut vm, resolver) = harness();
            let context = VmContext {
                read_once_input: input.as_slice(),
                immutable_input: &[],
                read_once_output: output.as_slice(),
                immutable_output: &[],
            };
            let res = vm
                .exec(lib.routine(FN_FUNGIBLE_TRANSFER), &context, resolver)
                .is_ok();
            if should_success {
                assert!(res);
            } else {
                assert!(!res);
            }
        }
    }

    #[test]
    fn transfer_deflation() {
        transfer_harness(&[&[1001], &[99, 900]], &[&[1000], &[100, 900]], false);
    }

    #[test]
    fn transfer_inflation() {
        transfer_harness(&[&[999], &[101, 900]], &[&[1000], &[100, 900]], false);
    }

    #[test]
    fn transfer_overflow() {
        transfer_harness(&[&[1]], &[&[u64::MAX - 1, 2]], false);
    }

    #[test]
    fn transfer_correct() {
        transfer_harness(&[&[1000], &[100, 900]], &[&[1000], &[100, 900]], true);
    }
}
