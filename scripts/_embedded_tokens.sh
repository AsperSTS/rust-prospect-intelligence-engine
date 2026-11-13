#!/bin/sh
# Helper compartido para cargar tokens únicamente cuando el usuario solicita
# de forma explícita un build personal con --embedded-tokens.
# Prohibido en CI o releases públicos.

clear_token_env() {
    unset DENUE_TOKEN INEGI_TOKEN BANXICO_TOKEN
}

parse_tokens_file() {
    tokens_path=$1
    if [ ! -f "$tokens_path" ]; then
        printf 'No se encontró el archivo de tokens: %s\n' "$tokens_path" >&2
        return 1
    fi
    while IFS= read -r line || [ -n "$line" ]; do
        line=$(printf '%s' "$line" | tr -d '\r')
        trimmed=$(printf '%s' "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        case "$trimmed" in
            ''|\#*) continue ;;
        esac
        case "$trimmed" in
            *=*)
                key=$(printf '%s' "${trimmed%%=*}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
                val=$(printf '%s' "${trimmed#*=}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
                case "$val" in
                    \"*\") val=${val#\"}; val=${val%\"} ;;
                    \'*\') val=${val#\'}; val=${val%\'} ;;
                esac
                case "$key" in
                    DENUE_TOKEN) if [ -n "$val" ]; then export DENUE_TOKEN="$val"; fi ;;
                    INEGI_TOKEN) if [ -n "$val" ]; then export INEGI_TOKEN="$val"; fi ;;
                    BANXICO_TOKEN) if [ -n "$val" ]; then export BANXICO_TOKEN="$val"; fi ;;
                esac
                ;;
        esac
    done < "$tokens_path"
}

load_embedded_tokens() {
    tokens_file=${1:-}
    if [ -n "${GITHUB_ACTIONS:-}" ] || [ -n "${CI:-}" ]; then
        printf '%s\n' 'Error: --embedded-tokens está prohibido en entornos CI o releases públicos.' >&2
        exit 1
    fi
    if [ -n "$tokens_file" ]; then
        clear_token_env
        parse_tokens_file "$tokens_file"
    elif [ -n "${DENUE_TOKEN:-}" ]; then
        :
    elif [ -f "./credentials.env" ]; then
        clear_token_env
        parse_tokens_file "./credentials.env"
    else
        default_home=${PROSPECTOS_HOME:-"$HOME/Library/Application Support/Prospect Intelligence"}
        if [ -f "$default_home/credentials.env" ]; then
            clear_token_env
            parse_tokens_file "$default_home/credentials.env"
        fi
    fi
    if [ -z "${DENUE_TOKEN:-}" ]; then
        printf '%s\n' 'Error: --embedded-tokens requiere DENUE_TOKEN (usa --tokens-file, env o credentials.env).' >&2
        exit 1
    fi
    printf '%s\n' 'AVISO: Compilando build PERSONAL con tokens embebidos. NO distribuir este binario públicamente.' >&2
}
