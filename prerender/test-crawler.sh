#!/bin/bash

# Test the pre-rendering system by simulating different user agents

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SERVER_URL=${1:-"http://localhost:8003"}

echo -e "${BLUE}Testing pre-rendering system at ${SERVER_URL}${NC}\n"

# Test with a regular user agent
echo -e "${GREEN}Testing with regular user agent:${NC}"
curl -s -o human.html -A "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.114 Safari/537.36" ${SERVER_URL}
echo "Regular user agent response saved to human.html"

# Test with Googlebot
echo -e "\n${GREEN}Testing with Googlebot user agent:${NC}"
curl -s -o googlebot.html -A "Googlebot/2.1 (+http://www.google.com/bot.html)" ${SERVER_URL}
echo "Googlebot response saved to googlebot.html"

# Compare file sizes to see if they're different
human_size=$(wc -c < human.html)
bot_size=$(wc -c < googlebot.html)

echo -e "\n${GREEN}Results:${NC}"
echo "Human response size: $human_size bytes"
echo "Bot response size: $bot_size bytes"

if [ "$human_size" != "$bot_size" ]; then
  echo -e "${GREEN}Success!${NC} Different responses are being served to bots vs humans."
else
  echo -e "${RED}Warning:${NC} Both responses are the same size. The bot detection might not be working."
fi

echo -e "\nYou can view the differences with: diff human.html googlebot.html" 