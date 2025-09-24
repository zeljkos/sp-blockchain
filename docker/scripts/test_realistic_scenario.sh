#!/bin/bash

echo "Testing realistic scenario (95% small, 5% large settlements)..."

# Simulate realistic SP telecom traffic pattern
total_records=20
large_records=$((total_records / 20))  # 5%
small_records=$((total_records - large_records))

# Generate small records
for i in $(seq 1 $small_records); do
    # Random small amounts between 50-99 EUR cents
    amount=$((RANDOM % 5000 + 5000))

    curl -X POST http://sp-bce-validator-1:8080/api/v1/bce/submit \
        -H "Content-Type: application/json" \
        -d '{
            "record_id": "BCE_REAL_SMALL_'$i'",
            "record_type": "voice_call",
            "imsi": "26201'$RANDOM'",
            "home_plmn": "26201",
            "visited_plmn": "23415",
            "session_duration": '$((RANDOM % 600 + 60))',
            "bytes_uplink": 0,
            "bytes_downlink": 0,
            "wholesale_charge": '$amount',
            "retail_charge": '$((amount * 140 / 100))',
            "currency": "EUR",
            "timestamp": '$(date +%s)'
        }' > /dev/null 2>&1

    echo -n "."
done

echo ""
echo "Generated $small_records small records"

# Generate large records
for i in $(seq 1 $large_records); do
    # Random large amounts between 100-500 EUR
    amount=$((RANDOM % 40000 + 10000))

    curl -X POST http://sp-bce-validator-1:8080/api/v1/bce/submit \
        -H "Content-Type: application/json" \
        -d '{
            "record_id": "BCE_REAL_LARGE_'$i'",
            "record_type": "data_session",
            "imsi": "26201'$RANDOM'",
            "home_plmn": "26201",
            "visited_plmn": "23415",
            "session_duration": '$((RANDOM % 7200 + 1800))',
            "bytes_uplink": '$((RANDOM % 50000000 + 10000000))',
            "bytes_downlink": '$((RANDOM % 200000000 + 50000000))',
            "wholesale_charge": '$amount',
            "retail_charge": '$((amount * 140 / 100))',
            "currency": "EUR",
            "timestamp": '$(date +%s)'
        }' | jq -r '.message'

    sleep 1
done

echo ""
echo "Generated $large_records large records"
echo "Realistic scenario test completed."