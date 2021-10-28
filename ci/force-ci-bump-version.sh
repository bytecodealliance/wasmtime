#!/bin/bash

if [ -z "$PERSONAL_ACCESS_TOKEN" ]; then
  echo "need to set \$PERSONAL_ACCESS_TOKEN to a github token"
  exit 1
fi

exec curl --include --request POST \
  https://api.github.com/repos/bytecodealliance/wasmtime/actions/workflows/bump-version.yml/dispatches \
  --header "Authorization: token $PERSONAL_ACCESS_TOKEN" \
  --data @- << EOF
{
  "ref": "main"
}
EOF

