#!/bin/bash

echo "Running SP BCE Consortium Tests..."

# Test API endpoints for all SP nodes
nodes=("sp-tmobile-de:8080" "sp-vodafone-uk:8080" "sp-orange-fr:8080" "sp-telenor-no:8080" "sp-sfr-fr:8080")

for node in "${nodes[@]}"; do
    echo "Testing $node health endpoint..."
    curl -f "http://$node/health" || echo "Failed to reach $node"
    echo ""
done

# Submit test BCE records with proper authentication and validation
echo "Submitting test BCE records..."

# T-Mobile-DE → Orange-FR roaming record
echo "Creating T-Mobile-DE record..."
curl -X POST "http://sp-tmobile-de:8080/api/v1/bce/submit" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer tmobile_api_key_2024_secure" \
    -d '{
        "record_id": "TMO-TEST-001",
        "imsi": "262010000000001",
        "home_operator": "T-Mobile-DE",
        "visited_operator": "Orange-FR",
        "call_minutes": 45,
        "data_mb": 1200,
        "sms_count": 8,
        "call_rate_cents": 15,
        "data_rate_cents": 5,
        "sms_rate_cents": 10,
        "wholesale_charge_cents": 6755,
        "timestamp": 1703097600,
        "roaming_minutes": 12,
        "roaming_data_mb": 300,
        "roaming_rate_cents": 25,
        "roaming_data_rate_cents": 8,
        "network_pair_hash": "TMO-ORA-TEST",
        "zkp_proof": null,
        "proof_verified": false,
        "consortium_signature": null
    }'

echo ""

# Vodafone-UK → T-Mobile-DE roaming record
echo "Creating Vodafone-UK record..."
curl -X POST "http://sp-vodafone-uk:8080/api/v1/bce/submit" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer vodafone_api_key_2024_secure" \
    -d '{
        "record_id": "VOD-TEST-001",
        "imsi": "234150000000001",
        "home_operator": "Vodafone-UK",
        "visited_operator": "T-Mobile-DE",
        "call_minutes": 32,
        "data_mb": 800,
        "sms_count": 5,
        "call_rate_cents": 18,
        "data_rate_cents": 6,
        "sms_rate_cents": 12,
        "wholesale_charge_cents": 5416,
        "timestamp": 1703097700,
        "roaming_minutes": 8,
        "roaming_data_mb": 200,
        "roaming_rate_cents": 30,
        "roaming_data_rate_cents": 10,
        "network_pair_hash": "VOD-TMO-TEST",
        "zkp_proof": null,
        "proof_verified": false,
        "consortium_signature": null
    }'

echo ""

# Orange-FR → Vodafone-UK roaming record
echo "Creating Orange-FR record..."
curl -X POST "http://sp-orange-fr:8080/api/v1/bce/submit" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer orange_api_key_2024_secure" \
    -d '{
        "record_id": "ORA-TEST-001",
        "imsi": "208010000000001",
        "home_operator": "Orange-FR",
        "visited_operator": "Vodafone-UK",
        "call_minutes": 28,
        "data_mb": 950,
        "sms_count": 3,
        "call_rate_cents": 12,
        "data_rate_cents": 7,
        "sms_rate_cents": 8,
        "wholesale_charge_cents": 6984,
        "timestamp": 1703097900,
        "roaming_minutes": 15,
        "roaming_data_mb": 250,
        "roaming_rate_cents": 20,
        "roaming_data_rate_cents": 9,
        "network_pair_hash": "ORA-VOD-TEST",
        "zkp_proof": null,
        "proof_verified": false,
        "consortium_signature": null
    }'

echo ""
echo "Consortium tests completed."