# GUFW

**Interface graphique simple, sobre et élégante pour gérer intégralement UFW et Fail2Ban — en local ou sur un ordinateur distant via SSH.**

Écrite en Rust pur (interface [egui](https://github.com/emilk/egui), client SSH [russh](https://github.com/Eugeny/russh) embarqué), distribuée en **AppImage** autonome.

---

## Fonctionnalités

### Pare-feu UFW
- État du pare-feu en un coup d'œil, activation/désactivation avec confirmation ;
- Politiques par défaut (entrée / sortie / routage : autoriser, refuser, rejeter) ;
- Niveau de journalisation (désactivé → complet) ;
- Règles : liste numérotée, recherche instantanée, ajout (port, plage, liste, service, profil d'application, adresse source/destination CIDR, position d'insertion, commentaire), suppression avec confirmation ;
- Rapport UFW, rechargement, réinitialisation (double confirmation par saisie).

### Fail2Ban
- État du service (démarrer / arrêter / redémarrer / recharger) ;
- Prisons : liste, détails (échecs, bannies, fichiers surveillés, bantime / findtime / maxretry) ;
- Bannir / débannir une IP, liste globale (« toutes les IP bannies ») ;
- Journaux du service (`journalctl -u fail2ban`) et blocages UFW (`journalctl -k --grep UFW`).

### Accès
- **Local** : une seule authentification root par session (polkit/`pkexec`), via un assistant minimal à liste blanche stricte ;
- **SSH** : mot de passe ou clé privée (avec phrase secrète), profils enregistrés, préfixe `sudo` optionnel, validation de la clé du serveur à la première connexion (TOFU) et alerte si elle change.

### Confort
- Journal intégré : chaque commande exécutée est horodatée et visible (filtres, copie) ;
- Actualisation automatique (3–60 s), notifications, annulation des bannissements/débannissements ;
- Thème sombre/clair, taille de texte réglable, raccourcis clavier (`Ctrl+1…5`, `Ctrl+R`, `Ctrl+N`).

## Sécurité (en bref)

| Mécanisme | Détail |
|---|---|
| Escalade root unique | Le premier accès aux outils lance `pkexec gufw --serve …` : **une** demande de mot de passe par session. L'assistant root n'exécute que `ufw`, `fail2ban-client`, `systemctl` et `journalctl` (unités `fail2ban`/`ufw` uniquement), validés par une liste blanche structurelle des deux côtés (interface + assistant). |
| Pas de shell | En local, exécution par argv direct. En SSH, chaque argument est échappé POSIX (`quote_shell`, testé unitairement). Aucune injection possible. |
| Socket UNIX privée | `/run/user/<uid>/gufw.sock`, permissions 0600, vérification `SO_PEERCRED`, arrêt automatique après inactivité. |
| Mots de passe | Jamais écrits sur le disque, effacés de la mémoire après connexion. |
| Clés de serveur | Empreinte SHA256 affichée à la 1re connexion, mémorisée, alerte forte si elle change. |

L'ergonomie complète (personas, parcours, système de design, décisions refusées) est étudiée dans **[docs/ERGONOMIE.md](docs/ERGONOMIE.md)**.

## Utilisation

```bash
# Rendre exécutable puis lancer :
chmod +x GUFW-0.1.0-x86_64.AppImage
./GUFW-0.1.0-x86_64.AppImage
```

- **Cet ordinateur** → *Se connecter* → saisissez votre mot de passe une fois (agent polkit de votre bureau) ;
- **SSH** → hôte, port, utilisateur, mot de passe ou clé, *Se connecter* (l'option « via sudo » convient aux connexions non-root).

## Compilation depuis les sources

```bash
# Prérequis : Rust (rustup), gcc (paquet build-essential)
cargo build --release            # binaire : target/release/gufw
cargo test                       # 24 tests unitaires (parseurs, whitelist, protocole)

# AppImage complet :
./packaging/build.sh             # → dist/GUFW-0.1.0-x86_64.AppImage
```

## Architecture

```
crates/
├── gufw-core/            # Aucune dépendance graphique — testable isolément
│   ├── exec.rs           # CommandSpec + liste blanche + échappement shell
│   ├── ufw.rs            # Commandes ufw + parseurs (status verbose/numbered…)
│   ├── fail2ban.rs       # Commandes fail2ban-client + parseurs tolérants
│   ├── local.rs          # Transport root direct
│   ├── helper.rs         # Assistant root (serveur socket UNIX + client)
│   ├── ssh.rs            # Transport russh (TOFU, sudo, auth mot de passe/clé)
│   ├── op.rs             # Opérations de haut niveau (Op → OpResult)
│   └── model.rs          # Règles, prisons, états…
└── gufw/                 # Interface egui
    ├── app.rs            # État, événements worker, opérations pendantes
    ├── worker.rs         # Thread async ↔ UI (journalisation de chaque commande)
    ├── theme.rs          # Palette sobre, typographie IBM Plex
    ├── ui/               # 5 écrans (connexion, pare-feu, fail2ban, journal, réglages)
    └── modal.rs          # Confirmations, ajout de règle, clé serveur…
```

## Limites connues (v1)

- Pas de réordonnancement glisser-déposer des règles (insertion par position disponible) ;
- Pas d'édition de `jail.local` par formulaire (gestion par les commandes officielles uniquement) ;
- La journalisation système en SSH suppose `systemd` sur la cible ;
- L'AppImage est construit sur glibc 2.43 (Debian 13/14) : pour une distribution plus ancienne, recompilez avec `packaging/build.sh`.

## Licence

MIT — voir les en-têtes des crates. Polices [IBM Plex](https://github.com/IBM/plex) (OFL) et [Material Icons](https://github.com/google/material-design-icons) (Apache 2.0) embarquées avec leurs licences respectives.
