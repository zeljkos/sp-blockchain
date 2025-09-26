// Trusted Setup Demo for 5-Party SP Consortium
// Pre-generates ZKP keys for all containers to use
use sp_blockchain::zkp::trusted_setup::TrustedSetupCeremony;
use ark_std::rand::thread_rng;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 SP 5-Party Consortium Trusted Setup Demo");

    // Create base keys directory for individual provider keys
    let base_keys_dir = PathBuf::from("./docker/zkp_keys");

    // Create directory if it doesn't exist
    tokio::fs::create_dir_all(&base_keys_dir).await?;

    println!("📁 Base keys directory: {:?}", base_keys_dir.canonicalize()?);

    println!("🏗️  Generating individual ZKP keys for each provider...");
    println!("   This approach is more realistic - each SP has their own keys");
    println!("   Providers: T-Mobile-DE, Vodafone-UK, Orange-FR, Telefónica-ES, SFR-FR");

    let mut rng = thread_rng();

    // Generate individual keys for each provider
    let provider_key_dirs = TrustedSetupCeremony::generate_individual_provider_keys(
        base_keys_dir.clone(),
        &mut rng
    ).await?;

    // For backward compatibility, also create a "shared" transcript
    let mut ceremony = TrustedSetupCeremony::sp_5node_consortium_ceremony(base_keys_dir.clone());
    let transcript = ceremony.run_ceremony(&mut rng).await?;

    println!("✅ Ceremony completed successfully!");
    println!("📋 Ceremony ID: {}", transcript.ceremony_id);
    println!("👥 Participants: {:?}", transcript.participants);
    println!("🔍 Verification Status: {:?}", transcript.verification_status);

    // Verify the ceremony
    let verification_result = ceremony.verify_ceremony().await?;
    println!("🔐 Ceremony verification: {}", verification_result);

    // Display ceremony completion details
    println!("📊 Individual Key Generation Statistics:");
    for (provider_id, keys_dir) in &provider_key_dirs {
        println!("   • {}: Keys at {:?}", provider_id, keys_dir);
    }

    println!("📊 Ceremony Statistics:");
    println!("   • CDR Privacy Circuit: Keys generated and verified");
    println!("   • Settlement Calculation Circuit: Keys generated and verified");

    // Test key loading for each provider to ensure they're working
    println!("🧪 Testing individual key loading for each provider...");

    for (provider_id, keys_dir) in &provider_key_dirs {
        let provider_ceremony = TrustedSetupCeremony::sp_5node_consortium_ceremony(keys_dir.clone());

        if provider_ceremony.keys_exist("cdr_privacy").await {
            let (_pk, _vk) = provider_ceremony.load_circuit_keys("cdr_privacy").await?;
            println!("🔑 Successfully loaded CDR Privacy circuit keys for {}", provider_id);
        }

        if provider_ceremony.keys_exist("settlement_calculation").await {
            let (_pk, _vk) = provider_ceremony.load_circuit_keys("settlement_calculation").await?;
            println!("🔑 Successfully loaded Settlement Calculation circuit keys for {}", provider_id);
        }
    }

    println!("🎉 Individual trusted setup complete! Each provider has their own keys.");
    println!("💡 Key structure:");
    println!("   docker/zkp_keys/tmobile-de/    - T-Mobile DE keys");
    println!("   docker/zkp_keys/vodafone-uk/   - Vodafone UK keys");
    println!("   docker/zkp_keys/orange-fr/     - Orange FR keys");
    println!("   docker/zkp_keys/telefonica-es/    - Telefónica ES keys");
    println!("   docker/zkp_keys/sfr-fr/        - SFR FR keys");
    println!("💡 You can now start the containers with: docker-compose up");

    Ok(())
}