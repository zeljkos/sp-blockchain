#!/bin/bash

echo "Testing large settlements (> 100 EUR)..."

# Large settlement BCE records that should trigger blockchain writes
for i in {1..3}; do
    echo "Submitting large BCE record $i..."

    curl -X POST http://sp-bce-validator-1:8080/api/v1/bce/submit \
        -H "Content-Type: application/json" \
        -d '{
            "record_id": "BCE_LARGE_'$i'",
            "record_type": "data_session",
            "imsi": "26201987654'$i'",
            "home_plmn": "26201",
            "visited_plmn": "23415",
            "session_duration": 3600,
            "bytes_uplink": 104857600,
            "bytes_downlink": 524288000,
            "wholesale_charge": 15000,
            "retail_charge": 25000,
            "currency": "EUR",
            "timestamp": '$(date +%s)'
        }' | jq '.'

    sleep 2
done

echo "Large settlements test completed."