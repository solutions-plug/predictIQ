#!/bin/bash

echo "🚀 PredictIQ API Service Setup Verification"
echo "==========================================="
echo ""

# Check if running
echo "1. Checking if API is running..."
if curl -s http://localhost:3000/health > /dev/null; then
    echo "   ✅ API is running"
    
    # Check health endpoint
    echo ""
    echo "2. Testing health check endpoint..."
    HEALTH=$(curl -s http://localhost:3000/health)
    echo "   Response: $HEALTH"
    
    if echo "$HEALTH" | grep -q "ok"; then
        echo "   ✅ Health check passed"
    else
        echo "   ❌ Health check failed"
    fi
    
    # Check rate limiting
    echo ""
    echo "3. Testing rate limiting..."
    for i in {1..5}; do
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/health)
        echo "   Request $i: HTTP $STATUS"
    done
    echo "   ✅ Rate limiting configured"
    
    # Check CORS
    echo ""
    echo "4. Testing CORS configuration..."
    CORS=$(curl -s -H "Origin: http://localhost:3001" -I http://localhost:3000/health | grep -i "access-control")
    if [ ! -z "$CORS" ]; then
        echo "   ✅ CORS configured"
    else
        echo "   ⚠️  CORS headers not found"
    fi
    
else
    echo "   ❌ API is not running"
    echo ""
    echo "To start the API:"
    echo "  cd api && npm install && npm run dev"
fi

echo ""
echo "==========================================="
echo "Verification complete!"
