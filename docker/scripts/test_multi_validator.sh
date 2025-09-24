#!/bin/bash

echo "Testing multi-validator settlement distribution..."

# Send settlements to different validators to test consensus
echo "Sending large settlement to Validator 1..."
curl -X POST http://sp-bce-validator-1:8080/api/v1/bce/submit \
    -H "Content-Type: application/json" \
    -d '{
        "record_id": "BCE_MULTI_V1",
        "record_type": "data_session",
        "imsi": "26201111111",
        "home_plmn": "26201",
        "visited_plmn": "23415",
        "session_duration": 3600,
        "bytes_uplink": 104857600,
        "bytes_downlink": 524288000,
        "wholesale_charge": 12000,
        "retail_charge": 18000,
        "currency": "EUR",
        "timestamp": '$(date +%s)'
    }' | jq '.'

sleep 3

echo "Sending large settlement to Validator 2..."
curl -X POST http://sp-bce-validator-2:8080/api/v1/bce/submit \
    -H "Content-Type: application/json" \
    -d '{
        "record_id": "BCE_MULTI_V2",
        "record_type": "data_session",
        "imsi": "26201222222",
        "home_plmn": "26201",
        "visited_plmn": "23415",
        "session_duration": 3600,
        "bytes_uplink": 104857600,
        "bytes_downlink": 524288000,
        "wholesale_charge": 11500,
        "retail_charge": 17000,
        "currency": "EUR",
        "timestamp": '$(date +%s)'
    }' | jq '.'

sleep 3

echo "Sending large settlement to Validator 3..."
curl -X POST http://sp-bce-validator-3:8080/api/v1/bce/submit \
    -H "Content-Type: application/json" \
    -d '{
        "record_id": "BCE_MULTI_V3",
        "record_type": "data_session",
        "imsi": "26201333333",
        "home_plmn": "26201",
        "visited_plmn": "23415",
        "session_duration": 3600,
        "bytes_uplink": 104857600,
        "bytes_downlink": 524288000,
        "wholesale_charge": 13000,
        "retail_charge": 19000,
        "currency": "EUR",
        "timestamp": '$(date +%s)'
    }' | jq '.'

echo "Multi-validator test completed."