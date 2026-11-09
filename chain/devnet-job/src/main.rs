//! Test d'un job complet sur Solana devnet, sans wallet. Un requester de test ouvre l'escrow,
//! soumet le job via l'API comme le site, puis attend le calcul, la vérification et le règlement
//! on-chain. Les nœuds (VPS, PC) font le reste.

mod api;
mod chain;
mod config;
mod job;

use api::{Api, address_of};
use chain::{OnChain, keypair_of};
use config::Config;
use sil_crypto::{Purpose, keyfile};
use sil_settlement::terms_of;
use sil_settlement_solana::{escrow_address, parse_program_id};
use sil_types::JobId;
use solana_sdk::signer::Signer;
use std::{process::ExitCode, thread::sleep, time::Duration};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("erreur : {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let cfg = Config::from_args(std::env::args().skip(1))?;
    if let Some(hex) = &cfg.read_job {
        return read_only(&cfg, hex);
    }
    let seed = keyfile::load_or_create(&cfg.seed).map_err(|e| e.to_string())?;
    let identity = seed.derive(Purpose::Ledger);
    println!("requester : {}", keypair_of(&seed).pubkey());

    let api = Api::login(&cfg.api, &identity)?;

    let input = payload_of(&cfg)?;
    let dimensions = (cfg.width, cfg.height);
    let built = job::build(
        &identity,
        &input,
        cfg.fragments,
        &cfg.workload,
        dimensions,
    )?;
    println!("job : {}", built.job.to_hex());
    if cfg.off_chain {
        // Réseau de test : pas d'escrow, donc pas de dépôt ni de règlement.
        api.upload(&input)?;
        api.publish(&built.job, &built.event)?;
        wait_finalized(&api, &built.job, cfg.timeout_s)?;
        println!("job finalisé hors chain (aucun escrow, aucun paiement)");
        return Ok(());
    }
    let terms = terms_of(&built.spec);
    let deposit = terms.max_payout().map_err(|e| format!("{e:?}"))?;
    println!("dépôt : {deposit} lamports");

    // Ordre du site : entrée, escrow, puis publication.
    let chain = OnChain::new(&cfg.rpc, &cfg.program, &seed, cfg.timeout_s)?;
    api.upload(&input)?;
    chain.open(&seed, &built.job, terms, deposit)?;
    println!("escrow ouvert, publication");
    api.publish(&built.job, &built.event)?;

    wait_finalized(&api, &built.job, cfg.timeout_s)?;
    wait_settled(&chain, &built.job, cfg.fragments, cfg.timeout_s)?;
    report(&chain, &built.job, cfg.fragments, deposit)
}

/// Entrée du job selon le workload : octets de test (`calcul/sha256`) ou image + scène
/// (`rendu/raytracer`). La scène doit annoncer la résolution demandée, sinon le rendu diverge.
fn payload_of(cfg: &Config) -> Result<Vec<u8>, String> {
    match cfg.workload.as_str() {
        "calcul/sha256" => Ok(job::input_bytes(cfg.bytes)),
        "rendu/raytracer" => {
            let path = cfg.scene.as_ref().ok_or("--scene requis")?;
            let scene = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let text = std::str::from_utf8(&scene).map_err(|_| "scène : UTF-8 attendu")?;
            let (width, height) = sil_raytracer::scene::scene_resolution(text);
            if (width, height) != (cfg.width as usize, cfg.height as usize) {
                return Err(format!(
                    "résolution de la scène {width}x{height} ≠ --width/--height {}x{}",
                    cfg.width, cfg.height
                ));
            }
            Ok(job::image_payload(cfg.width, cfg.height, &scene))
        }
        other => Err(format!("workload inconnu : {other}")),
    }
}

/// Mode lecture : adresse du compte d'escrow et son état on-chain, sans rien envoyer.
fn read_only(cfg: &Config, hex: &str) -> Result<(), String> {
    let job = JobId::from_hex(hex).map_err(|e| e.to_string())?;
    let program = parse_program_id(&cfg.program).map_err(|e| format!("{e:?}"))?;
    println!("compte d'escrow : {}", escrow_address(&program, &job));
    let chain = OnChain::new_readonly(&cfg.rpc, &cfg.program, cfg.timeout_s)?;
    match chain.escrow(&job)? {
        None => println!("escrow : absent"),
        Some(escrow) => {
            println!(
                "escrow : présent, payé {} lamports, fragments réglés {}",
                escrow.paid(),
                escrow.settled_count()
            );
            // Bénéficiaires des fragments réglés (les fragments sont numérotés depuis 0).
            for fragment in 0..16 {
                let Some(settled) = escrow.settled(fragment) else {
                    continue;
                };
                println!("fragment {fragment} : réalisé par {}", address_of(&settled.claimant));
                for approver in &settled.approvers {
                    println!("  vérifié par {}", address_of(approver));
                }
            }
        }
    }
    Ok(())
}

/// Attend « finalized » côté réseau ; échoue si le job est annulé ou expiré.
fn wait_finalized(api: &Api, job: &JobId, timeout_s: u64) -> Result<(), String> {
    let started = std::time::Instant::now();
    let mut last = String::new();
    loop {
        let state = api.state(job)?;
        if state != last {
            println!("état du job : {state}");
            last = state.clone();
        }
        match state.as_str() {
            "finalized" => return Ok(()),
            "cancelled" | "expired" => return Err(format!("job {state}")),
            _ => {}
        }
        if started.elapsed() > Duration::from_secs(timeout_s) {
            return Err("délai dépassé : finalisation du job".into());
        }
        sleep(Duration::from_secs(3));
    }
}

/// Attend que chaque fragment soit réglé on-chain.
fn wait_settled(
    chain: &OnChain,
    job: &JobId,
    fragments: u32,
    timeout_s: u64,
) -> Result<(), String> {
    let started = std::time::Instant::now();
    loop {
        let settled = chain
            .escrow(job)?
            .map_or(0, |escrow| escrow.settled_count());
        if settled == fragments as usize {
            println!("règlement : {settled}/{fragments} fragments");
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(timeout_s) {
            return Err(format!(
                "règlement incomplet : {settled}/{fragments} fragments"
            ));
        }
        sleep(Duration::from_secs(3));
    }
}

/// Montants réglés : total payé et, par fragment, qui a été payé.
fn report(chain: &OnChain, job: &JobId, fragments: u32, deposit: u64) -> Result<(), String> {
    let escrow = chain.escrow(job)?.ok_or("escrow introuvable")?;
    println!("payé : {} / {deposit} lamports", escrow.paid());
    for fragment in 0..fragments {
        let settled = escrow.settled(fragment).ok_or("fragment non réglé")?;
        println!(
            "fragment {fragment} : réalisé par {}",
            address_of(&settled.claimant)
        );
        for approver in &settled.approvers {
            println!("  vérifié par {}", address_of(approver));
        }
    }
    Ok(())
}
