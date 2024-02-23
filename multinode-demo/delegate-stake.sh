#!/usr/bin/env bash

# shellcheck disable=SC1090
# shellcheck disable=SC1091
source "$(dirname "$0")"/get_program_accounts.sh

usage() {
    exitcode=0
    if [[ -n "$1" ]]; then
        exitcode=1
        echo "Error: $*"
    fi
  cat <<EOF
usage: $0 [cluster_rpc_url]

 Report total token distribution of a running cluster owned by the following programs:
   STAKE
   SYSTEM
   VOTE
   CONFIG

 Required arguments:
   cluster_rpc_url  - RPC URL and port for a running Nexis cluster (ex: http://34.83.146.144:8899)
EOF
    exit $exitcode
}

function get_cluster_version {
    clusterVersion="$(curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1, "method":"getVersion"}' "$url" | jq '.result | ."nexis-core" ')"
    echo Cluster software version: "$clusterVersion"
}

function get_token_capitalization {
    totalSupplyLamports="$(curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1, "method":"getTotalSupply"}' "$url" | cut -d , -f 2 | cut -d : -f 2)"
    totalSupplySol=$((totalSupplyLamports / LAMPORTS_PER_NZT))
    
    printf "\n--- Token Capitalization ---\n"
    printf "Total token capitalization %'d NZT\n" "$totalSupplySol"
    printf "Total token capitalization %'d Lamports\n" "$totalSupplyLamports"
    
}

function get_program_account_balance_totals {
    PROGRAM_NAME="$1"
    
    # shellcheck disable=SC2002
    accountBalancesLamports="$(cat "${PROGRAM_NAME}_account_data.json" | \
jq '.result | .[] | .account | .lamports')"

totalAccountBalancesLamports=0
numberOfAccounts=0

# shellcheck disable=SC2068
for account in ${accountBalancesLamports[@]}; do
    totalAccountBalancesLamports=$((totalAccountBalancesLamports + account))
    numberOfAccounts=$((numberOfAccounts + 1))
done
totalAccountBalancesSol=$((totalAccountBalancesLamports / LAMPORTS_PER_NZT))

printf "\n--- %s Account Balance Totals ---\n" "$PROGRAM_NAME"
printf "Number of %s Program accounts: %'.f\n" "$PROGRAM_NAME" "$numberOfAccounts"
printf "Total token balance in all %s accounts: %'d NZT\n" "$PROGRAM_NAME" "$totalAccountBalancesSol"
printf "Total token balance in all %s accounts: %'d Lamports\n" "$PROGRAM_NAME" "$totalAccountBalancesLamports"

case $PROGRAM_NAME in
    SYSTEM)
        systemAccountBalanceTotalSol=$totalAccountBalancesSol
        systemAccountBalanceTotalLamports=$totalAccountBalancesLamports
    ;;
    STAKE)
        stakeAccountBalanceTotalSol=$totalAccountBalancesSol
        stakeAccountBalanceTotalLamports=$totalAccountBalancesLamports
    ;;
    VOTE)
        voteAccountBalanceTotalSol=$totalAccountBalancesSol
        voteAccountBalanceTotalLamports=$totalAccountBalancesLamports
    ;;
    CONFIG)
        configAccountBalanceTotalSol=$totalAccountBalancesSol
        configAccountBalanceTotalLamports=$totalAccountBalancesLamports
    ;;
    *)
        echo "Unknown program: $PROGRAM_NAME"
        exit 1
    ;;
esac
}

function sum_account_balances_totals {
grandTotalAccountBalancesSol=$((systemAccountBalanceTotalSol + stakeAccountBalanceTotalSol + voteAccountBalanceTotalSol + configAccountBalanceTotalSol))
grandTotalAccountBalancesLamports=$((systemAccountBalanceTotalLamports + stakeAccountBalanceTotalLamports + voteAccountBalanceTotalLamports + configAccountBalanceTotalLamports))

printf "\n--- Total Token Distribution in all Account Balances ---\n"
printf "Total NZT in all Account Balances: %'d\n" "$grandTotalAccountBalancesSol"
printf "Total Lamports in all Account Balances: %'d\n" "$grandTotalAccountBalancesLamports"
}

url=$1
[[ -n $url ]] || usage "Missing required RPC URL"
shift

LAMPORTS_PER_NZT=1000000000 # 1 billion

stakeAccountBalanceTotalSol=
systemAccountBalanceTotalSol=
voteAccountBalanceTotalSol=
configAccountBalanceTotalSol=

stakeAccountBalanceTotalLamports=
systemAccountBalanceTotalLamports=
voteAccountBalanceTotalLamports=
configAccountBalanceTotalLamports=

echo "--- Querying RPC URL: $url ---"
get_cluster_version

get_program_accounts STAKE "$STAKE_PROGRAM_PUBKEY" "$url"
get_program_accounts SYSTEM "$SYSTEM_PROGRAM_PUBKEY" "$url"
get_program_accounts VOTE "$VOTE_PROGRAM_PUBKEY" "$url"
get_program_accounts CONFIG "$CONFIG_PROGRAM_PUBKEY" "$url"

write_program_account_data_csv STAKE
write_program_account_data_csv SYSTEM
write_program_account_data_csv VOTE
write_program_account_data_csv CONFIG

get_token_capitalization

get_program_account_balance_totals STAKE
get_program_account_balance_totals SYSTEM
get_program_account_balance_totals VOTE
get_program_account_balance_totals CONFIG

common_args+=(--url "$url")

if [[ ${#positional_args[@]} -gt 1 ]]; then
  usage "$@"
fi
if [[ -n ${positional_args[0]} ]]; then
  stake_sol=${positional_args[0]}
fi

VALIDATOR_KEYS_DIR=$NZT_CONFIG_DIR/validator$label
vote_account="${vote_account:-$VALIDATOR_KEYS_DIR/vote-account.json}"
stake_account="${stake_account:-$VALIDATOR_KEYS_DIR/stake-account.json}"

if [[ ! -f $vote_account ]]; then
  echo "Error: $vote_account not found"
  exit 1
fi

if ((airdrops_enabled)); then
  if [[ -z $keypair ]]; then
    echo "--keypair argument must be provided"
    exit 1
  fi
  $exzo_cli \
    "${common_args[@]}" --keypair "$NZT_CONFIG_DIR/faucet.json" \
    transfer --allow-unfunded-recipient "$keypair" "$stake_sol"
fi

if [[ -n $keypair ]]; then
  common_args+=(--keypair "$keypair")
fi

if ! [[ -f "$stake_account" ]]; then
  $exzo_keygen new --no-passphrase -so "$stake_account"
else
  echo "$stake_account already exists! Using it"
fi

set -x
$exzo_cli "${common_args[@]}" \
  vote-account "$vote_account"
$exzo_cli "${common_args[@]}" \
  create-stake-account "$stake_account" "$stake_sol"
$exzo_cli "${common_args[@]}" \
  delegate-stake $maybe_force "$stake_account" "$vote_account"
$exzo_cli "${common_args[@]}" stakes "$stake_account"