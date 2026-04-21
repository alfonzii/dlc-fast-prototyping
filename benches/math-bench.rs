use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dlc_fast_prototyping::config::runparams::{MyAdaptorSignatureScheme, MyCryptoUtils};
use rand::thread_rng;
use secp256k1_zkp::{Keypair, Secp256k1};
use serde_json::Value;
use std::{env, fs, path::PathBuf};

// Import necessary types and functions
use dlc_fast_prototyping::adaptor_signature_scheme::AdaptorSignatureScheme;
use dlc_fast_prototyping::common::fun; // contains create_cet and create_message
use dlc_fast_prototyping::common::types::OutcomeU32;
use dlc_fast_prototyping::crypto_utils::CryptoUtils;

const POW2_20SUB1: u32 = 1_048_575; // twenty bits set to 1 in binary
const POW2_10SUB1: u32 = 1023; // ten bits set to 1 in binary

fn criterion_root_dir() -> PathBuf {
    match env::var("CARGO_TARGET_DIR") {
        Ok(target_dir) => PathBuf::from(target_dir).join("criterion"),
        Err(_) => PathBuf::from("target").join("criterion"),
    }
}

fn read_criterion_mean_ns(benchmark_id: &str) -> Option<f64> {
    let estimates_path = criterion_root_dir()
        .join(benchmark_id)
        .join("new")
        .join("estimates.json");

    let json_raw = fs::read_to_string(estimates_path).ok()?;
    let value: Value = serde_json::from_str(&json_raw).ok()?;

    value.get("mean")?.get("point_estimate")?.as_f64()
}

fn format_ns(ns: f64) -> String {
    if ns >= 1_000_000.0 {
        format!("{:.3}ms", ns / 1_000_000.0)
    } else if ns >= 1_000.0 {
        format!("{:.3}us", ns / 1_000.0)
    } else {
        format!("{:.3}ns", ns)
    }
}

fn report_selected_ratio_from_criterion(_c: &mut Criterion) {
    let selected = [
        ("bench_create_cet", "create_cet"),
        ("bench_create_message", "create_message"),
        (
            "bench_compute_anticipation_point",
            "compute_anticipation_point",
        ),
        ("bench_pre_sign", "pre_sign"),
    ];

    let mut measured = Vec::with_capacity(selected.len());
    for (label, bench_id) in selected {
        match read_criterion_mean_ns(bench_id) {
            Some(mean_ns) => measured.push((label, mean_ns)),
            None => {
                println!(
                    "\nCould not read Criterion mean for benchmark '{}'.",
                    bench_id
                );
                println!("Relative ratio report skipped.\n");
                return;
            }
        }
    }

    let total: f64 = measured.iter().map(|(_, mean_ns)| *mean_ns).sum();

    println!("\n=============================================================");
    println!("Relative Runtime Report (Criterion means, selected only)");
    println!("-------------------------------------------------------------");
    println!("{:<35}{:<15}{:<15}", "FUNCTION", "MEAN", "RATIO");
    println!("-------------------------------------------------------------");

    for (label, mean_ns) in &measured {
        let ratio = if total == 0.0 {
            0.0
        } else {
            (*mean_ns / total) * 100.0
        };
        println!("{:<35}{:<15}{:.2}%", label, format_ns(*mean_ns), ratio);
    }

    println!("-------------------------------------------------------------");
    println!(
        "{:<35}{:<15}{}",
        "TOTAL (4 selected functions)",
        format_ns(total),
        "100.00%"
    );
    println!("=============================================================\n");
}

fn bench_create_cet(c: &mut Criterion) {
    let total_collateral = 1000;
    let payout = 400;
    c.bench_function("create_cet", |b| {
        b.iter(|| {
            let cet = black_box(fun::create_cet(payout, total_collateral));
            black_box(cet)
        })
    });
}

fn bench_create_message(c: &mut Criterion) {
    let cet_str = "Alice gets 600 sats and Bob gets 400 sats".to_string();
    c.bench_function("create_message", |b| {
        b.iter(|| {
            let msg = black_box(fun::create_message(&cet_str)).unwrap();
            black_box(msg)
        })
    });
}

fn bench_compute_anticipation_point(c: &mut Criterion) {
    let secp = Secp256k1::new();
    let (_, oracle_pub) = secp.generate_keypair(&mut thread_rng());
    let (_, oracle_nonce) = secp.generate_keypair(&mut thread_rng());
    let crypto_utils_engine = MyCryptoUtils::new(&oracle_pub, &oracle_nonce);
    let outcome = OutcomeU32::from(POW2_10SUB1);
    c.bench_function("compute_anticipation_point", |b| {
        b.iter(|| {
            let atp = black_box(crypto_utils_engine.compute_anticipation_point(&outcome)).unwrap();
            black_box(atp)
        })
    });
}

fn bench_pre_sign(c: &mut Criterion) {
    let secp = Secp256k1::new();
    let keypair = Keypair::new(&secp, &mut thread_rng());
    let cet_str = "Alice gets 600 sats and Bob gets 400 sats".to_string();
    let msg = fun::create_message(&cet_str).unwrap();
    // For anticipation point, generate dummy keys:
    let (_, oracle_pub) = secp.generate_keypair(&mut thread_rng());
    let (_, oracle_nonce) = secp.generate_keypair(&mut thread_rng());
    let crypto_utils_engine = MyCryptoUtils::new(&oracle_pub, &oracle_nonce);
    let outcome = OutcomeU32::from(POW2_10SUB1);
    let atp_point = crypto_utils_engine
        .compute_anticipation_point(&outcome)
        .unwrap();
    c.bench_function("pre_sign", |b| {
        b.iter(|| {
            let _ = MyAdaptorSignatureScheme::pre_sign(&keypair, &msg, &atp_point);
        })
    });
}

fn bench_verify_adaptor(c: &mut Criterion) {
    use dlc_fast_prototyping::adaptor_signature_scheme::AdaptorSignatureScheme;
    use rand::thread_rng;
    use secp256k1_zkp::Secp256k1;

    let secp = Secp256k1::new();
    let keypair = Keypair::new(&secp, &mut thread_rng());
    let (_, oracle_pk) = secp.generate_keypair(&mut thread_rng());
    let (_, oracle_nonce) = secp.generate_keypair(&mut thread_rng());

    let crypto_utils_engine = MyCryptoUtils::new(&oracle_pk, &oracle_nonce);

    let outcome = OutcomeU32::from(POW2_10SUB1);
    let cet_str = fun::create_cet(400, 1000);
    let msg = fun::create_message(&cet_str).unwrap();
    let atp_point = crypto_utils_engine
        .compute_anticipation_point(&outcome)
        .unwrap();

    let adaptor_sig = MyAdaptorSignatureScheme::pre_sign(&keypair, &msg, &atp_point);
    c.bench_function("verify_adaptor_sig", |b| {
        b.iter(|| {
            let _check = black_box(MyAdaptorSignatureScheme::pre_verify(
                &keypair.public_key(),
                &msg,
                &atp_point,
                &adaptor_sig,
            ));
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10000);
    targets = bench_create_cet, bench_create_message, bench_compute_anticipation_point, bench_pre_sign, bench_verify_adaptor, report_selected_ratio_from_criterion
}
criterion_main!(benches);
