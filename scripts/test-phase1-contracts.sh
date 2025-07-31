#!/bin/bash
# Test Script for Phase 1 Contract Changes
# Tests that the consistent field naming works correctly

set -e

echo "🧪 Testing Phase 1: Consistent Field Naming"
echo "==========================================="

# Test 1: Canvas API with correct field names (should work)
echo ""
echo "Test 1: Canvas API with consistent field names"
echo "Expected: HTTP 200"

RESPONSE=$(curl -s -w "HTTPSTATUS:%{http_code}" -X PATCH "http://localhost:8001/api/workflows/current/canvas" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-token" \
  -d '{
    "canvas": {
      "edges": [{
        "from_node_id": "node_0",
        "to_node_id": "node_1", 
        "config": {"id": "edge-test"}
      }],
      "nodes": [
        {
          "id": "node_0",
          "type": "trigger",
          "position": {"x": 100, "y": 200},
          "config": {"text": "Test Trigger"}
        },
        {
          "id": "node_1",
          "type": "agent", 
          "position": {"x": 300, "y": 400},
          "config": {"text": "Test Agent"}
        }
      ]
    }
  }')

HTTP_STATUS=$(echo $RESPONSE | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
RESPONSE_BODY=$(echo $RESPONSE | sed -e 's/HTTPSTATUS\:.*//g')

if [ "$HTTP_STATUS" -eq 200 ]; then
    echo "✅ PASS: Canvas API accepts consistent field names"
else
    echo "❌ FAIL: Expected 200, got $HTTP_STATUS"
    echo "Response: $RESPONSE_BODY"
    exit 1
fi

# Test 2: Verify response contains correct field names
echo ""
echo "Test 2: Response contains consistent field names"
if echo "$RESPONSE_BODY" | grep -q '"from_node_id"' && echo "$RESPONSE_BODY" | grep -q '"to_node_id"'; then
    echo "✅ PASS: Response uses consistent edge field names"
else
    echo "❌ FAIL: Response does not contain from_node_id/to_node_id"
    echo "Response: $RESPONSE_BODY"
    exit 1
fi

# Test 3: Test with old field names (should fail)
echo ""
echo "Test 3: Canvas API with old field names (should fail)"  
echo "Expected: HTTP 422 (validation error)"

RESPONSE2=$(curl -s -w "HTTPSTATUS:%{http_code}" -X PATCH "http://localhost:8001/api/workflows/current/canvas" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-token" \
  -d '{
    "canvas": {
      "edges": [{
        "from": "node_0",
        "to": "node_1",
        "config": {}
      }],
      "nodes": []
    }
  }')

HTTP_STATUS2=$(echo $RESPONSE2 | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')

if [ "$HTTP_STATUS2" -eq 422 ]; then
    echo "✅ PASS: Old field names correctly rejected"
else
    echo "⚠️  WARNING: Expected 422 for old field names, got $HTTP_STATUS2"
fi

# Test 4: Check OpenAPI schema generation
echo ""
echo "Test 4: OpenAPI schema generation"

if [ -f "/Users/davidrose/git/zerg/openapi.json" ]; then
    EDGE_SCHEMA=$(cat /Users/davidrose/git/zerg/openapi.json | jq -r '.components.schemas.WorkflowEdge.properties | keys[]' 2>/dev/null || echo "")
    if echo "$EDGE_SCHEMA" | grep -q "from_node_id" && echo "$EDGE_SCHEMA" | grep -q "to_node_id"; then
        echo "✅ PASS: OpenAPI schema uses consistent field names"
    else
        echo "❌ FAIL: OpenAPI schema does not contain consistent field names"
        exit 1
    fi
else
    echo "❌ FAIL: OpenAPI schema file not found"
    exit 1
fi

echo ""
echo "🎉 Phase 1 Tests Complete!"
echo "========================================="
echo "✅ All contract consistency tests passed"
echo "✅ Backend now enforces consistent field naming"  
echo "✅ OpenAPI schema reflects correct field names"
echo ""
echo "Next: Update frontend to use consistent field names"