#!/usr/bin/env bash
# Erzeugt das Delta-Paket für ein Update des FüKw (IDA-332): statt des ganzen Images (~3,6 MB)
# lädt der Server nur Patches der geänderten Ebenen. Gegenstück: deploy-delta in
# rubenvitt/fuekw-deploy, dort steht auch das Format.
#   scripts/image-delta.sh <basis>@sha256:… <ziel>@sha256:… <os/arch> <ausgabeordner>
# Exit 0: Paket liegt im Ausgabeordner. Exit 3: kein Delta sinnvoll (nichts geändert, Patch nicht
# kleiner, Basis unbekannt) – dann zieht der Server ganz. Sonst: Fehler.
# Zugang zur Registry: anonym, oder REGISTRY_USER/REGISTRY_PASSWORD.
set -euo pipefail

[[ $# -eq 4 ]] || { echo "Aufruf: $0 <basis>@sha256:… <ziel>@sha256:… <os/arch> <ausgabeordner>" >&2; exit 2; }
base_ref=$1 target_ref=$2 plat=$3 out=$4
skip() { echo "Kein Delta: $*" >&2; exit 3; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

sha() { sha256sum "$1" | cut -d' ' -f1; }

# Registry-Zugriff mit Bearer-Token nach WWW-Authenticate (ghcr.io, Docker Hub, registry:2 ohne Auth)
declare -A tokens
token() {  # token <host> <name>
  local key="$1/$2" hdr realm service
  if [[ -z ${tokens[$key]+x} ]]; then
    hdr=$(curl -sS -o /dev/null -D - "https://$1/v2/" | tr -d '\r' | grep -i '^www-authenticate: bearer' || true)
    if [[ -z $hdr ]]; then
      tokens[$key]=
    else
      realm=$(sed -E 's/.*realm="([^"]+)".*/\1/' <<< "$hdr")
      service=$(sed -nE 's/.*service="([^"]+)".*/\1/p' <<< "$hdr")
      auth=()
      [[ -z ${REGISTRY_USER:-} ]] || auth=(-u "$REGISTRY_USER:$REGISTRY_PASSWORD")
      tokens[$key]=$(curl -fsS "${auth[@]}" -G "$realm" --data-urlencode "service=$service" \
        --data-urlencode "scope=repository:$2:pull" | jq -r '.token // .access_token')
    fi
  fi
  echo "${tokens[$key]}"
}
# get <host> <name> <manifests|blobs> <digest> <datei>: lädt und prüft den Digest
get() {
  local t
  t=$(token "$1" "$2")
  curl -fsSL ${t:+-H "Authorization: Bearer $t"} \
    -H 'Accept: application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json' \
    "https://$1/v2/$2/$3/$4" -o "$5"
  [[ "sha256:$(sha "$5")" == "$4" ]] || { echo "Digest von $2 $4 stimmt nicht" >&2; exit 1; }
}
unpack() {  # unpack <mediaType> <datei> → unkomprimiert auf stdout
  case $1 in
    *tar+zstd) zstd -dcq "$2" ;;
    *tar.gzip|*tar+gzip) gzip -dc "$2" ;;
    *tar) cat "$2" ;;
    *) echo "unbekannter Ebenentyp $1" >&2; exit 1 ;;
  esac
}
# fetch <ref> <ordner>: index.json (falls Index), manifest.json, config.json der Plattform
fetch() {
  local ref=$1 dir=$2 host rest name digest mt m
  [[ $ref =~ ^([^/]+)/([^@]+)@(sha256:[0-9a-f]{64})$ ]] || { echo "ungültige Referenz $ref" >&2; exit 2; }
  host=${BASH_REMATCH[1]} name=${BASH_REMATCH[2]} digest=${BASH_REMATCH[3]}
  mkdir -p "$dir"
  echo "$host $name" > "$dir/repo"
  get "$host" "$name" manifests "$digest" "$dir/top.json"
  mt=$(jq -r '.mediaType // ""' "$dir/top.json")
  if [[ $mt == *index* || $mt == *manifest.list* ]]; then
    mv "$dir/top.json" "$dir/index.json"
    m=$(jq -r --arg p "$plat" '[.manifests[] | select("\(.platform.os)/\(.platform.architecture)" == $p)] | first | .digest // ""' "$dir/index.json")
    [[ -n $m ]] || { echo "keine Plattform $plat in $ref" >&2; exit 1; }
    get "$host" "$name" manifests "$m" "$dir/manifest.json"
  else
    mv "$dir/top.json" "$dir/manifest.json"
  fi
  get "$host" "$name" blobs "$(jq -r .config.digest "$dir/manifest.json")" "$dir/config.json"
}
# layer <ordner> <i> <ziel>: Ebene i unkomprimiert nach <ziel>, gegen ihre diff_id geprüft
layer() {
  local host name d mt
  read -r host name < "$1/repo"
  d=$(jq -r ".layers[$2].digest" "$1/manifest.json")
  mt=$(jq -r ".layers[$2].mediaType" "$1/manifest.json")
  get "$host" "$name" blobs "$d" "$tmp/blob"
  unpack "$mt" "$tmp/blob" > "$3"
  rm -f "$tmp/blob"
  [[ "sha256:$(sha "$3")" == "$(jq -r ".rootfs.diff_ids[$2]" "$1/config.json")" ]] \
    || { echo "diff_id der Ebene $2 stimmt nicht" >&2; exit 1; }
}

[[ $target_ref == *@sha256:* && $base_ref == *@sha256:* ]] || skip "Basis oder Ziel ohne Digest"
fetch "$base_ref" "$tmp/base"
fetch "$target_ref" "$tmp/target"
[[ -f $tmp/target/index.json ]] || { echo "Ziel $target_ref ist kein Index" >&2; exit 1; }

mapfile -t base_diff < <(jq -r '.rootfs.diff_ids[]' "$tmp/base/config.json")
mapfile -t target_diff < <(jq -r '.rootfs.diff_ids[]' "$tmp/target/config.json")
rm -rf "$out"; mkdir -p "$out/patches"
patches=() patch_bytes=0 full_bytes=0
for i in "${!target_diff[@]}"; do
  id=${target_diff[$i]}
  printf '%s\n' "${base_diff[@]}" | grep -qxF "$id" && continue
  # Gegenstück: die Ebene an derselben Stelle, sonst die letzte der Basis
  j=$i; (( j < ${#base_diff[@]} )) || j=$(( ${#base_diff[@]} - 1 ))
  layer "$tmp/base" "$j" "$tmp/old.tar"
  layer "$tmp/target" "$i" "$tmp/new.tar"
  file="${id#sha256:}.zst"
  zstd -q -19 --long=27 --patch-from="$tmp/old.tar" "$tmp/new.tar" -o "$out/patches/$file" 2>/dev/null
  # Probe wie am Server
  zstd -dq --memory=512MB --patch-from="$tmp/old.tar" "$out/patches/$file" -o "$tmp/check.tar"
  cmp -s "$tmp/check.tar" "$tmp/new.tar" || { echo "Patch für $id lässt sich nicht anwenden" >&2; exit 1; }
  rm -f "$tmp/old.tar" "$tmp/new.tar" "$tmp/check.tar"
  patches+=("$(jq -nc --arg d "$id" --arg f "${base_diff[$j]}" --arg file "$file" '{diff_id: $d, from: $f, file: $file}')")
  patch_bytes=$(( patch_bytes + $(stat -c %s "$out/patches/$file") ))
  full_bytes=$(( full_bytes + $(jq -r ".layers[$i].size" "$tmp/target/manifest.json") ))
done
(( ${#patches[@]} > 0 )) || skip "keine geänderte Ebene"
(( patch_bytes * 10 < full_bytes * 8 )) || skip "Patch ($patch_bytes B) kaum kleiner als die Ebenen ($full_bytes B)"

cp "$tmp/target/index.json" "$tmp/target/manifest.json" "$tmp/target/config.json" "$out/"
printf '%s\n' "${patches[@]}" | jq -s --arg image "$target_ref" --arg base "$base_ref" --arg p "$plat" \
  '{format: 1, image: $image, base: $base, platform: $p, patches: .}' > "$out/delta.json"
echo "Delta: ${#patches[@]} Ebene(n), $(( patch_bytes / 1024 )) KB statt $(( full_bytes / 1024 )) KB"
