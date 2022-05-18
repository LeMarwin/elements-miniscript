// Miniscript
// Written in 2019 by
//     Andrew Poelstra <apoelstra@wpsoftware.net>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Example: Signing a 2-of-3 multisignature

extern crate elements_miniscript as miniscript;

use crate::miniscript::DescriptorTrait;
use elements::secp256k1_zkp;
use std::collections::HashMap;
use std::str::FromStr;

fn main() {
    // Avoid repeatedly typing a pretty-common descriptor type
    type BitcoinDescriptor = miniscript::Descriptor<bitcoin::PublicKey>;

    // Transaction which spends some output
    let mut tx = elements::Transaction {
        version: 2,
        lock_time: 0,
        input: vec![elements::TxIn {
            previous_output: elements::OutPoint::default(),
            script_sig: elements::Script::new(),
            sequence: 0xffffffff,
            is_pegin: false,
            has_issuance: false,
            asset_issuance: elements::AssetIssuance::default(),
            witness: elements::TxInWitness::default(),
        }],
        output: vec![elements::TxOut {
            script_pubkey: elements::Script::new(),
            value: elements::confidential::Value::Explicit(100_000_000),
            witness: elements::TxOutWitness::default(),
            asset: elements::confidential::Asset::default(),
            nonce: elements::confidential::Nonce::default(),
        }],
    };

    #[cfg_attr(feature="cargo-fmt", rustfmt_skip)]
    let public_keys = vec![
        bitcoin::PublicKey::from_slice(&[2; 33]).expect("key 1"),
        bitcoin::PublicKey::from_slice(&[
            0x02,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]).expect("key 2"),
        bitcoin::PublicKey::from_slice(&[
            0x03,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]).expect("key 3"),
    ];
    let bitcoin_sig = (
        // copied at random off the blockchain; this is not actually a valid
        // signature for this transaction; Miniscript does not verify
        secp256k1_zkp::ecdsa::Signature::from_str(
            "3045\
             0221\
             00f7c3648c390d87578cd79c8016940aa8e3511c4104cb78daa8fb8e429375efc1\
             0220\
             531d75c136272f127a5dc14acc0722301cbddc222262934151f140da345af177",
        )
        .unwrap(),
        elements::EcdsaSigHashType::All,
    );

    let descriptor_str = format!(
        "elwsh(multi(2,{},{},{}))",
        public_keys[0], public_keys[1], public_keys[2],
    );

    // Descriptor for the output being spent
    let my_descriptor =
        BitcoinDescriptor::from_str(&descriptor_str[..]).expect("parse descriptor string");

    // Check weight for witness satisfaction cost ahead of time.
    // 4(scriptSig length of 0) + 1(witness stack size) + 106(serialized witnessScript)
    // + 73*2(signature length + signatures + sighash bytes) + 1(dummy byte) = 258
    assert_eq!(my_descriptor.max_satisfaction_weight().unwrap(), 258);

    // Sometimes it is necessary to have additional information to get the bitcoin::PublicKey
    // from the MiniscriptKey which can supplied by `to_pk_ctx` parameter. For example,
    // when calculating the script pubkey of a descriptor with xpubs, the secp context and
    // child information maybe required.

    // Observe the script properties, just for fun
    assert_eq!(
        format!("{:x}", my_descriptor.script_pubkey()),
        "00200ed49b334a12c37f3df8a2974ad91ff95029215a2b53f78155be737907f06163"
    );

    assert_eq!(
        format!(
            "{:x}",
            my_descriptor
                .explicit_script()
                .expect("wsh descriptors have unique inner script")
        ),
        "52\
         21020202020202020202020202020202020202020202020202020202020202020202\
         21020102030405060708010203040506070801020304050607080000000000000000\
         21030102030405060708010203040506070801020304050607080000000000000000\
         53ae"
    );

    // Attempt to satisfy at age 0, height 0
    let original_txin = tx.input[0].clone();

    let mut sigs = HashMap::<bitcoin::PublicKey, miniscript::ElementsSig>::new();

    // Doesn't work with no signatures
    assert!(my_descriptor.satisfy(&mut tx.input[0], &sigs).is_err());
    assert_eq!(tx.input[0], original_txin);

    // ...or one signature...
    sigs.insert(public_keys[1], bitcoin_sig);
    assert!(my_descriptor.satisfy(&mut tx.input[0], &sigs).is_err());
    assert_eq!(tx.input[0], original_txin);

    // ...but two signatures is ok
    sigs.insert(public_keys[2], bitcoin_sig);
    assert!(my_descriptor.satisfy(&mut tx.input[0], &sigs).is_ok());
    assert_ne!(tx.input[0], original_txin);
    assert_eq!(tx.input[0].witness.script_witness.len(), 4); // 0, sig, sig, witness script

    // ...and even if we give it a third signature, only two are used
    sigs.insert(public_keys[0], bitcoin_sig);
    assert!(my_descriptor.satisfy(&mut tx.input[0], &sigs).is_ok());
    assert_ne!(tx.input[0], original_txin);
    assert_eq!(tx.input[0].witness.script_witness.len(), 4); // 0, sig, sig, witness script
}
