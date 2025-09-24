#!/bin/bash

echo "🚀 Testing SP BCE Consortium with Realistic Telecom Records"
echo "=========================================================="

# BCE Record Test Cases
echo
echo "📱 Scenario 1: German tourist roaming in UK (T-Mobile DE → Vodafone UK)"
curl -X POST http://localhost:8081/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "TMO_UK_001_20250922_143501",
    "record_type": "voice_call",
    "imsi": "262011234567890",
    "home_plmn": "26201",
    "visited_plmn": "23415",
    "session_duration": 420,
    "bytes_uplink": 0,
    "bytes_downlink": 0,
    "wholesale_charge": 2850,
    "retail_charge": 3500,
    "currency": "EUR",
    "timestamp": 1758559701,
    "charging_id": 1001
  }' && echo

echo
echo "📊 Scenario 2: French business data roaming in Germany (Orange FR → T-Mobile DE)"
curl -X POST http://localhost:8082/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "ORA_DE_002_20250922_144215",
    "record_type": "data_session",
    "imsi": "208011987654321",
    "home_plmn": "20801",
    "visited_plmn": "26201",
    "session_duration": 3600,
    "bytes_uplink": 52428800,
    "bytes_downlink": 157286400,
    "wholesale_charge": 4200,
    "retail_charge": 5000,
    "currency": "EUR",
    "timestamp": 1758560135,
    "charging_id": 1002
  }' && echo

echo
echo "💬 Scenario 3: Norwegian SMS roaming in France (Telenor NO → Orange FR)"
curl -X POST http://localhost:8083/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "TEL_FR_003_20250922_145033",
    "record_type": "sms",
    "imsi": "242011122334455",
    "home_plmn": "24201",
    "visited_plmn": "20801",
    "session_duration": 0,
    "bytes_uplink": 160,
    "bytes_downlink": 0,
    "wholesale_charge": 450,
    "retail_charge": 650,
    "currency": "EUR",
    "timestamp": 1758560633,
    "charging_id": 1003
  }' && echo

echo
echo "🎥 Scenario 4: UK high-data streaming in Norway (Vodafone UK → Telenor NO)"
curl -X POST http://localhost:8084/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "VOD_NO_004_20250922_150247",
    "record_type": "data_session",
    "imsi": "234151555666777",
    "home_plmn": "23415",
    "visited_plmn": "24201",
    "session_duration": 7200,
    "bytes_uplink": 10485760,
    "bytes_downlink": 2147483648,
    "wholesale_charge": 8900,
    "retail_charge": 12000,
    "currency": "EUR",
    "timestamp": 1758561767,
    "charging_id": 1004
  }' && echo

echo
echo "☎️ Scenario 5: French emergency call in UK (SFR FR → Vodafone UK)"
curl -X POST http://localhost:8085/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "SFR_UK_005_20250922_151130",
    "record_type": "emergency_call",
    "imsi": "208102333444555",
    "home_plmn": "20810",
    "visited_plmn": "23415",
    "session_duration": 180,
    "bytes_uplink": 0,
    "bytes_downlink": 0,
    "wholesale_charge": 0,
    "retail_charge": 0,
    "currency": "EUR",
    "timestamp": 1758562290,
    "charging_id": 1005
  }' && echo

echo
echo "🔄 Scenario 6: Large data session triggering settlement (German in France)"
curl -X POST http://localhost:8081/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "TMO_FR_006_20250922_152415",
    "record_type": "data_session",
    "imsi": "262019876543210",
    "home_plmn": "26201",
    "visited_plmn": "20801",
    "session_duration": 10800,
    "bytes_uplink": 104857600,
    "bytes_downlink": 5368709120,
    "wholesale_charge": 15600,
    "retail_charge": 20000,
    "currency": "EUR",
    "timestamp": 1758563055,
    "charging_id": 1006
  }' && echo

echo
echo "📞 Scenario 7: Multiple calls to reach settlement threshold"
echo "Call 1/3:"
curl -X POST http://localhost:8082/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "VOD_DE_007a_20250922_153201",
    "record_type": "voice_call",
    "imsi": "234159988776655",
    "home_plmn": "23415",
    "visited_plmn": "26201",
    "session_duration": 1200,
    "bytes_uplink": 0,
    "bytes_downlink": 0,
    "wholesale_charge": 6800,
    "retail_charge": 8500,
    "currency": "EUR",
    "timestamp": 1758563521,
    "charging_id": 1007
  }' && echo

echo "Call 2/3:"
curl -X POST http://localhost:8082/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "VOD_DE_007b_20250922_154033",
    "record_type": "voice_call",
    "imsi": "234159988776655",
    "home_plmn": "23415",
    "visited_plmn": "26201",
    "session_duration": 900,
    "bytes_uplink": 0,
    "bytes_downlink": 0,
    "wholesale_charge": 5100,
    "retail_charge": 6400,
    "currency": "EUR",
    "timestamp": 1758564033,
    "charging_id": 1008
  }' && echo

echo "Call 3/3 (Should trigger settlement at 100 EUR threshold):"
curl -X POST http://localhost:8082/api/v1/bce/submit \
  -H "Content-Type: application/json" \
  -d '{
    "record_id": "VOD_DE_007c_20250922_155117",
    "record_type": "voice_call",
    "imsi": "234159988776655",
    "home_plmn": "23415",
    "visited_plmn": "26201",
    "session_duration": 600,
    "bytes_uplink": 0,
    "bytes_downlink": 0,
    "wholesale_charge": 3400,
    "retail_charge": 4200,
    "currency": "EUR",
    "timestamp": 1758564677,
    "charging_id": 1009
  }' && echo

echo
echo "📊 Checking Settlement Statistics Across SP Consortium:"
echo "======================================================="
echo
echo "🇩🇪 T-Mobile DE Settlement Stats:"
curl -s http://localhost:8081/api/v1/bce/stats | jq .
echo
echo "🇬🇧 Vodafone UK Settlement Stats:"
curl -s http://localhost:8082/api/v1/bce/stats | jq .
echo
echo "🇫🇷 Orange FR Settlement Stats:"
curl -s http://localhost:8083/api/v1/bce/stats | jq .
echo
echo "🇳🇴 Telenor NO Settlement Stats:"
curl -s http://localhost:8084/api/v1/bce/stats | jq .
echo
echo "🇫🇷 SFR FR Settlement Stats:"
curl -s http://localhost:8085/api/v1/bce/stats | jq .

echo
echo "✅ BCE Record Testing Complete!"
echo "The SP consortium has processed realistic telecom roaming scenarios"
echo "including voice calls, data sessions, SMS, and emergency calls."
echo "Some records should have triggered automatic settlements via blockchain."