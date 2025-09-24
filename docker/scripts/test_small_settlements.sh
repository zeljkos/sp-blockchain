#!/bin/bash

echo "Testing small settlements (< 100 EUR)..."

# Small settlement BCE records that should stay in memory
for i in {1..5}; do
    echo "Submitting small BCE record $i..."

    curl -X POST http://sp-bce-validator-1:8080/api/v1/bce/submit \
        -H "Content-Type: application/json" \
        -d '{
            "record_id": "BCE_SMALL_'$i'",
            "record_type": "voice_call",
            "imsi": "26201123456'$i'",
            "home_plmn": "26201",
            "visited_plmn": "23415",
            "session_duration": 180,
            "bytes_uplink": 0,
            "bytes_downlink": 0,
            "wholesale_charge": 850,
            "retail_charge": 1200,
            "currency": "EUR",
            "timestamp": '$(date +%s)'
        }' | jq '.'

    sleep 1
done

echo "Small settlements test completed."