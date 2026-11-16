#!/usr/bin/env bash
# Crée le compte de déploiement d'un environnement et installe le helper root. À lancer une fois, en root
# (ou via sudo), depuis la racine du dépôt. Idempotent.
#
#   sudo bash infra/vps/deploy-user.sh <prod|dev> <fichier de la clé publique du CI>
#
# Le compte est silicium-deploy-<env> : connexion par clé seulement (mot de passe verrouillé), pas de
# shell d'administration. Son seul droit root est d'exécuter le helper pour SON environnement :
#   silicium-deploy-dev ALL=(root) NOPASSWD: /usr/local/sbin/silicium-deploy dev *
# Un compte dev ne peut donc rien faire sur la prod, et inversement.
set -euo pipefail

env_name="${1:?usage : deploy-user.sh <prod|dev> <clé publique>}"
pubkey_file="${2:?clé publique manquante}"

case "$env_name" in
  prod|dev) ;;
  *) echo "environnement inconnu : $env_name" >&2; exit 2 ;;
esac
[[ "$(id -u)" -eq 0 ]] || { echo "Lancer en root." >&2; exit 1; }
[[ -f "$pubkey_file" ]] || { echo "clé publique absente : $pubkey_file" >&2; exit 1; }

key="$(tr -d '\r' < "$pubkey_file" | head -n1)"
case "$key" in
  ssh-ed25519\ *|ecdsa-sha2-nistp256\ *) ;;
  *) echo "la clé doit être une clé publique OpenSSH (ssh-ed25519 ...)" >&2; exit 2 ;;
esac

user="silicium-deploy-$env_name"
home="/home/$user"

if ! id "$user" >/dev/null 2>&1; then
  useradd --create-home --home-dir "$home" --shell /bin/bash --comment "Silicium deploy $env_name" "$user"
fi
passwd -l "$user" >/dev/null

install -d -o "$user" -g "$user" -m 0700 "$home/.ssh"
# Options : pas de redirection de ports, d'agent ni de X11 : le compte ne sert qu'à déployer.
printf 'no-port-forwarding,no-agent-forwarding,no-X11-forwarding %s\n' "$key" > "$home/.ssh/authorized_keys"
chown "$user:$user" "$home/.ssh/authorized_keys"
chmod 0600 "$home/.ssh/authorized_keys"

# Helper root : copie depuis le dépôt, en root, puis droits fixes.
helper_src="$(cd "$(dirname "$0")" && pwd)/silicium-deploy-helper.sh"
install -o root -g root -m 0755 "$helper_src" /usr/local/sbin/silicium-deploy

sudoers="/etc/sudoers.d/$user"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
printf '%s ALL=(root) NOPASSWD: /usr/local/sbin/silicium-deploy %s *\n' "$user" "$env_name" > "$tmp"
visudo -cf "$tmp" >/dev/null
install -o root -g root -m 0440 "$tmp" "$sudoers"

echo "compte prêt : $user (environnement $env_name)"
echo "Vérifier : sudo -l -U $user"
