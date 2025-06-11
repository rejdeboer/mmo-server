#!/bin/sh
curl -X POST http://localhost:9200/_security/api_key -H 'Content-Type: application/json' -d \
'{
  "name": "backend"
}'
