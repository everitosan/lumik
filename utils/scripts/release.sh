#!/usr/bin/env bash
#
# Release de la app desktop de Lumik.
#
# Calcula la próxima versión (bump del patch del último tag), permite
# sobrescribirla a mano, valida el CHANGELOG y crea+empuja el tag `vX.Y.Z`
# que dispara el workflow .github/workflows/release.yml (build Linux + Windows).
#
# Uso:
#   utils/scripts/release.sh              # sugiere versión y pregunta
#   utils/scripts/release.sh v0.2.0-beta  # versión explícita
#   VERSION=v0.2.0-beta utils/scripts/release.sh
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# ── Sugerencia: bump del patch del último tag ──────────────────────────────
last_tag="$(git tag -l 'v*' --sort=-v:refname | head -1)"
if [[ -z "$last_tag" ]]; then
  suggested="v0.1.0-beta"
else
  core="${last_tag#v}"            # v0.1.2-beta -> 0.1.2-beta
  suffix=""
  if [[ "$core" == *-* ]]; then    # separa el pre-release (-beta, -rc.1, ...)
    suffix="-${core#*-}"
    core="${core%%-*}"
  fi
  IFS=. read -r major minor patch <<< "$core"
  suggested="v${major}.${minor}.$((patch + 1))${suffix}"
fi

# ── Versión: argumento > $VERSION > prompt interactivo ─────────────────────
version="${1:-${VERSION:-}}"
if [[ -z "$version" ]]; then
  read -rp "Versión a liberar [${suggested}]: " version
  version="${version:-$suggested}"
fi
[[ "$version" == v* ]] || version="v$version"

echo "→ Último tag:    ${last_tag:-(ninguno)}"
echo "→ Nueva versión: $version"

# ── Validaciones ───────────────────────────────────────────────────────────
branch="$(git branch --show-current)"
[[ "$branch" == "main" ]] || echo "⚠  No estás en main (rama actual: $branch)"

if git rev-parse "$version" >/dev/null 2>&1; then
  echo "✖  El tag $version ya existe. Aborta." >&2
  exit 1
fi

ver_no_v="${version#v}"
if ! grep -q "^## \[${ver_no_v}\]" CHANGELOG.md; then
  echo "⚠  CHANGELOG.md no tiene la entrada '## [${ver_no_v}]'."
  echo "   El release se publicaría SIN notas. Agrégala antes de continuar."
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "⚠  Hay cambios sin commitear; el tag apuntará a HEAD ($(git rev-parse --short HEAD))."
fi

# ── Confirmación y disparo ─────────────────────────────────────────────────
read -rp "¿Crear y empujar el tag $version? Dispara el release en GitHub Actions [y/N]: " ok
[[ "$ok" == [yY] ]] || { echo "Cancelado."; exit 0; }

git tag -a "$version" -m "Lumik $version"
git push origin "$version"

echo "✔  Tag $version empujado. Sigue el build en la pestaña Actions del repo."
