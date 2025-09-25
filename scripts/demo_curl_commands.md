# 🚀 Smart Contract Demo - Manual curl Commands

## For Telefonica Demo Call

These curl commands demonstrate the smart contract deployment and execution capabilities of the SP Blockchain system.

## 🔑 Authentication

All API calls require authentication with a valid SP API key:

```bash
API_KEY="tmobiledeapikey2024secure"
BASE_URL="http://localhost:8081"  # T-Mobile DE node
```

## 📋 1. Check System Health

```bash
curl -X GET "$BASE_URL/health" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" | jq '.'
```

## 📊 2. Get Contract Statistics

```bash
curl -X GET "$BASE_URL/api/v1/contracts/stats" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" | jq '.'
```

## 🚀 3. Deploy Smart Contracts

### Contract 1: BCE Validation Contract (ZKP-powered)

```bash
curl -X POST "$BASE_URL/api/v1/contracts/deploy" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "contract_id": "telefonica-demo-bce-validator",
    "contract_type": "bce_validator",
    "operators": ["tmobile-de", "vodafone-uk", "orange-fr", "telenor-no", "sfr-fr"],
    "description": "Validates BCE records using real Groth16 ZKP proofs for privacy-preserving settlement verification"
  }' | jq '.'
```

### Contract 2: Multilateral Netting Contract

```bash
curl -X POST "$BASE_URL/api/v1/contracts/deploy" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "contract_id": "telefonica-demo-netting-contract",
    "contract_type": "netting_contract",
    "operators": ["tmobile-de", "vodafone-uk", "orange-fr", "telenor-no", "sfr-fr"],
    "description": "5-party multilateral netting achieving ~75% reduction in bilateral settlements"
  }' | jq '.'
```

### Contract 3: Settlement Execution Contract

```bash
curl -X POST "$BASE_URL/api/v1/contracts/deploy" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "contract_id": "telefonica-demo-settlement-executor",
    "contract_type": "settlement_executor",
    "operators": ["tmobile-de", "vodafone-uk", "orange-fr", "telenor-no", "sfr-fr"],
    "description": "Executes final settlements with multi-party digital signatures and dispute resolution"
  }' | jq '.'
```

## 📋 4. List All Deployed Contracts

```bash
curl -X GET "$BASE_URL/api/v1/contracts/list" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" | jq '.'
```

## ⚡ 5. Execute Contract Methods

### BCE Rate Validation

```bash
curl -X POST "$BASE_URL/api/v1/contracts/execute" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "contract_id": "telefonica-demo-bce-validator",
    "method": "validate_bce_rates",
    "parameters": {
      "call_rate_cents": 25,
      "data_rate_cents": 8,
      "sms_rate_cents": 12
    }
  }' | jq '.'
```

### Settlement Execution (with ZKP)

```bash
curl -X POST "$BASE_URL/api/v1/contracts/execute" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "contract_id": "telefonica-demo-settlement-executor",
    "method": "execute_settlement",
    "parameters": {
      "settlement_id": "TELEFONICA_DEMO_001",
      "total_amount_cents": 150000,
      "operators": ["tmobile-de", "vodafone-uk", "orange-fr"],
      "generate_zkp": true,
      "private_amounts": [50000, 60000, 40000],
      "private_rates": [25, 30, 20]
    }
  }' | jq '.'
```

### Get Contract Statistics

```bash
curl -X POST "$BASE_URL/api/v1/contracts/execute" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "contract_id": "telefonica-demo-bce-validator",
    "method": "get_stats",
    "parameters": {}
  }' | jq '.'
```

## 🎯 Demo Flow for Telefonica Call

1. **Show system health** - Demonstrate the SP blockchain is operational
2. **Deploy 3 contracts** - Show real-time contract deployment
3. **List contracts** - Verify successful deployment
4. **Execute validation** - Demo ZKP-powered BCE validation
5. **Show statistics** - Display performance metrics

## 🌐 Key Points for Telefonica

- **Real ZKP Integration**: Uses Groth16 proofs for privacy
- **5-Party Consortium**: Ready for multi-operator environments
- **Production-Ready**: Full authentication and security
- **Scalable Architecture**: Can handle Telefonica's volume
- **Custom Contracts**: Extensible for Telefonica-specific rules

## 🔧 Technical Highlights

- **Zero-Knowledge Proofs**: Groth16 with BN254 elliptic curves
- **Smart Contract VM**: Gas-metered execution environment
- **Multi-Signature Support**: Secure multi-party settlements
- **Real-Time API**: REST endpoints for all operations
- **Persistent Storage**: RocksDB for blockchain data
- **P2P Networking**: Distributed consensus across nodes

---

*Ready for Telefonica integration and custom contract development!*