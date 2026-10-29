# GUFW — Étude ergonomique de l'interface

> Interface graphique simple, sobre, élégante et conviviale pour gérer intégralement
> **UFW** et **Fail2Ban**, en local ou à distance via SSH.

---

## 1. Contexte et objectifs

### 1.1 Ce que l'application doit permettre

| Domaine | Besoin utilisateur |
|---|---|
| **UFW** | Connaître l'état du pare-feu d'un coup d'œil ; l'activer/désactiver ; régler les politiques par défaut ; ajouter/supprimer des règles (port, protocole, IP source/destination, profil d'application, commentaire) ; régler la journalisation ; voir le rapport ; réinitialiser. |
| **Fail2Ban** | Savoir si le service tourne ; lister les prisons (jails) ; voir les IP bannies ; bannir / débannir une IP ; consulter les paramètres d'une prison (bantime, findtime, maxretry) ; démarrer/arrêter/redémarrer le service ; recharger une prison ; consulter les journaux système. |
| **Accès** | Travailler en local (la machine où l'app tourne) ou sur un ordinateur distant via SSH (profil de connexion enregistré). |
| **Confiance** | Voir *ce que l'application fait* : chaque commande passée au système est journalisée, visible et compréhensible. |

### 1.2 Utilisateurs cibles (personas)

- **P1 — L'utilisatrice prudente.** Elle a installé Ubuntu sur son laptop, a entendu dire
  qu'il faut activer le pare-feu. Elle veut *une* action évidente : « activer », puis
  autoriser l'application qu'elle vient d'installer. Le vocabulaire technique doit rester
  compréhensible, les valeurs par défaut sûres (politique entrante : refuser).
- **P2 — L'administrateur système.** Il veut de la précision : CIDR, protocole, position
  d'insertion d'une règle, sorties brutes en cas d'erreur, réactivité (< 200 ms pour
  une action locale). Il déteste les interfaces qui masquent ou reformulent les erreurs.
- **P3 — L'admin à distance.** Elle gère un serveur VPS par SSH. Déploiement en 2 minutes :
  choisir un profil, saisir le mot de passe, voir l'état du serveur. Elle veut surtout
  débannir sa propre IP après 3 échecs de connexion, et bannir un attaquant.
- **P4 — L'auditeur.** Il veut voir le journal des actions de l'application et le rapport
  UFW, pour comprendre l'historique et expliquer un incident.

### 1.3 Contraintes techniques qui influencent l'ergonomie

- UFW et Fail2Ban exigent **root** : l'application doit rendre cette barrière
  *visible mais peu coûteuse* (une authentification par session, pas par clic).
- En SSH, la latence est variable : chaque action doit donner un **retour immédiat**
  d'envoi (bouton verrouillé, indicateur de synchronisation) puis un résultat.
- Les commandes système échouent parfois (ufw absent, service arrêté, droits refusés) :
  l'erreur système doit être affichée telle quelle, avec une piste d'action.

---

## 2. Principes directeurs

1. **Simplicité progressive.** L'écran par défaut montre l'essentiel en 3 cartes.
   Les options avancées (IP source/destination, position d'insertion, commentaire)
   sont révélées par une seule action (« Options avancées »), jamais imposées.
2. **État toujours visible.** Un point de couleur + une phrase courte décrivent l'état
   réel du pare-feu et de Fail2Ban, rafraîchis automatiquement (5 s par défaut,
   réglable, désactivable). Aucun chiffre affiché n'est jamais périmé de plus de 10 s
   sans que l'utilisateur le sache (indicateur « il y a X s »).
3. **Sécurité lisible.** Le vert signifie « protégé/actif », le rouge « désactivé/
   danger », l'ambre « attention ». Ces couleurs ne servent **jamais** de décoration.
   Une action destructive demande confirmation *une* fois, avec un texte de conséquence.
4. **Transparence.** Chaque commande système exécutée est enregistrée dans le
   **Journal** avec horodatage, commande exacte, code de retour et extrait de sortie.
   Aucune action silencieuse.
5. **Tolérance à l'erreur.** Les erreurs affichent le message du système + une
   suggestion (« Fail2Ban n'est pas installé sur la cible » → lien vers l'aide).
   Les actions réversibles (bannir/débannir) affichent un toast avec bouton « Annuler ».
6. **Sobriété.** Une seule teinte d'accent, deux polices (IBM Plex Sans pour
   l'interface, IBM Plex Mono pour les IP, ports et commandes), aucun emoji,
   aucune animation décorative. Le rythme visuel vient de l'espacement (grille de 8 px),
   pas de la couleur.

---

## 3. Parcours utilisateurs

### US1 — Premier lancement, machine locale (P1)
1. L'application s'ouvre sur l'écran **Connexion** : deux grandes cartes, « Cette
   machine » (sélectionnée par défaut) et « À distance (SSH) ».
2. Choix « Cette machine » → bouton principal unique : **Se connecter**.
3. L'application a besoin des droits root : elle affiche **une** boîte d'information
   expliquant qu'une demande de mot de passe va apparaître une seule fois pour la
   session. `pkexec` demande le mot de passe (agent polkit du bureau).
4. Connexion établie → l'écran **Pare-feu** s'affiche : grande carte « Pare-feu
   actif/désactivé » avec interrupteur. Si inactif : bandeau discret « Votre pare-feu
   est désactivé » + bouton Activer en vert.
5. P1 active → toast « Pare-feu activé », état visuel passé au vert, journalisation
   automatique de la commande.

### US2 — Autoriser une application (P1)
1. Onglet Pare-feu → bouton « + Ajouter une règle » (coin supérieur droit de la carte
   Règles) ou `Ctrl+N`.
2. Modale compacte : **Autoriser / Bloquer / Rejeter** (segmenté), champ port
   (mono), protocole (TCP / UDP / les deux), et « Options avancées » repliées.
3. Le champ port accepte `8080`, `8000:8010`, `22,80,443`, ou un nom de service
   (`ssh`), ou un profil d'application (liste déroulante issue de `ufw app list`).
   Validation en direct : bordure rouge + message si invalide, bouton Ajouter
   désactivé tant que invalide.
4. Ajouter → modale se ferme, toast « Règle ajoutée », la liste se rafraîchit,
   la nouvelle règle apparaît brièvement surlignée.

### US3 — Règle experte (P2)
1. Dans la modale d'ajout, « Options avancées » révèle : adresse source (CIDR),
   adresse destination, position d'insertion, commentaire, sens (entrée/sortie).
2. Ajout → le champ commentaire apparaît dans la table (colonne Commentaire) ;
   les IP et CIDR sont rendus en **police mono** pour être comparables d'une ligne à l'autre.
3. P2 survole une règle → apparition d'icônes discrètes : supprimer, détail.
4. Supprimer → modale légère « Supprimer la règle 22/tcp (SSH) ? » avec le
   bouton danger en bas-droite (jamais au milieu du texte).

### US4 — Diagnostiquer une attaque et se débannir (P3)
1. Écran Connexion → carte « À distance (SSH) » → choisir un profil enregistré
   (ou saisir hôte/port/utilisateur, mot de passe ou clé).
2. Première connexion : l'application affiche l'**empreinte SHA256** de la clé du
   serveur et demande confirmation (« Clé inconnue — vérifiez cette empreinte auprès
   de votre hébergeur »). Après confirmation, elle est mémorisée. Un changement
   d'empreinte ultérieur déclenche un avertissement fort (usurpation possible).
3. Onglet **Fail2Ban** : cartes « Service : actif », « Prisons : 3 », « IP bannies : 12 ».
4. Clic sur la prison `sshd` → tableau des IP bannies (mono), avec pour chacune
   un bouton débannir ; champ « Bannir une IP » au-dessus du tableau.
5. Elle clique « Débannir » sur sa propre IP → toast avec bouton « Annuler »
   (re-bannir immédiatement).

### US5 — Comprendre une action passée (P4)
1. Onglet **Journal** : liste chronologique horodatée : heure, action, commande
   exacte (mono), résultat (code + badge).
2. Filtres : Tout / Erreurs / Pare-feu / Fail2Ban ; bouton « Copier » pour
   exporter l'entrée vers le presse-papier (utile pour un ticket).

### US6 — Administration hors-ligne / erreur (P2)
1. Le serveur distant ne répond plus → l'indicateur de connexion passe à l'ambre
   « Reconnexion… » puis rouge « Déconnecté ». Les zones de données se figent
   (données grisées + mention « dernier état il y a 3 min »), les boutons
   d'action sont désactivés.
2. Un bouton « Se reconnecter » apparaît dans la carte de connexion ; aucune perte
   de saisie en cours (la modale d'ajout de règle reste ouverte).

---

## 4. Architecture de l'interface

### 4.1 Structure générale

```
┌──────────┬──────────────────────────────────────────────────────────┐
│          │  ● Connecté — Local (root)            ⟳ auto   ⟳   ⚙      │  barre sup.
│  GUFW    ├──────────────────────────────────────────────────────────┤
│          │                                                          │
│ Connexion│   [ cartes d'état — visibles à chaque écran du domaine ]  │
│          │                                                          │
│ Pare-feu │   [ zone principale : tableaux, actions ]                │
│          │                                                          │
│ Fail2Ban │                                                          │
│          │                                                          │
│ Journal  │                                                          │
│          │                                                          │
│ Paramètres│                                                          │
└──────────┴──────────────────────────────────────────────────────────┘
```

- **Barre latérale** (220 px, repliable à 64 px en icônes) : identité de l'app + 5
  entrées. L'entrée active est marquée par une pilule d'accent, pas par une couleur.
- **Barre supérieure** : toujours visible ; donne le **contexte** (quelle machine
  suis-je en train de toucher ?) — point vert (local) / bleu (SSH, nom du serveur
  en mono) / rouge (déconnecté), bouton actualiser, interrupteur auto-actualisation,
  réglages. C'est la seule barre toujours identique : elle répond à la question
  la plus fréquente en administration distante (« suis-je bien sur le bon serveur ? »).
- **Zone contenu** : cartes à coins arrondis (12 px), fond légèrement plus clair
  que le fond de page, ombre nulle (sobriété), titre de carte en 12 px majuscules
  espacées + actions à droite.
- **Barre d'état** en bas (24 px) : dernier événement + compteur d'actions en cours.

### 4.2 Écran « Connexion »

```
┌──────────────────────────────────────────────┐
│  Où voulez-vous agir ?                        │
│                                              │
│  ┌─────────────────┐  ┌─────────────────┐    │
│  │ ☑ CETTE MACHINE │  │   À DISTANCE    │    │
│  │                 │  │   (SSH)         │    │
│  │ UFW et Fail2Ban │  │  hôte port user │    │
│  │ de cet ordinate.│  │  profil: [v]    │    │
│  │ [ Se connecter ]│  │ [ Se connecter ]│    │
│  └─────────────────┘  └─────────────────┘    │
│                                              │
│  i Les mots de passe ne sont jamais          │
│    enregistrés. Droits root demandés une     │
│    seule fois par session (polkit).          │
└──────────────────────────────────────────────┘
```

- Carte locale : détecte et affiche si UFW/Fail2Ban sont installés *avant* la
  connexion (retour immédiat, sans droits) → évite l'erreur tardive.
- Carte SSH : liste des profils enregistrés ; « + Nouveau profil » ; le mot de
  passe est un champ de session, jamais persisté ; option « exécuter via sudo »
  pour les connexions sans utilisateur root.
- Après connexion, cet écran devient un récapitulatif (hôte, version des outils
  détectées, durée de session, bouton Déconnecter).

### 4.3 Écran « Pare-feu » (UFW)

```
┌ Pare-feu ────────────────────────────────────────────────────────┐
│                                                                  │
│  ┌ Pare-feu ─────────┐ ┌ Politiques par défaut ─┐ ┌ Journal ───┐ │
│  │ ● ACTIF            │ │ Entrée   [Refuser v]  │ │ ● Faible   │ │
│  │ UFW 0.36.2         │ │ Sortie   [Autoriser v]│ │ [Niveau v] │ │
│  │ [ Interrupteur ]   │ │ Routage  [Refuser v]  │ │            │ │
│  └────────────────────┘ └───────────────────────┘ └────────────┘ │
│                                                                  │
│  ┌ Règles (12) ────────────────────────────── [+ Ajouter] [⟳] ┐ │
│  │ #  Action    De           Vers       Port    Proto  Comment. │ │
│  │ 1  Autoriser Partout      Partout   22/tcp  —      SSH      │ │
│  │ 2  Autoriser 10.0.0.0/8   Partout   80,443  tcp    Web      │ │
│  │ 3  Bloquer   Partout      192.168.1.5 —       —      —       │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  [ Rapport UFW ]  [ Réinitialiser… ]   ← actions rares, en bas   │
└──────────────────────────────────────────────────────────────────┘
```

- **Carte Pare-feu** : le composant le plus important de l'app. Un interrupteur
  grand (44 px), vert si actif, gris si inactif ; phrase d'état en clair.
  Désactiver → modale de confirmation (conséquences expliquées).
- **Carte Politiques** : trois listes (Autoriser / Refuser / Rejeter).
  Le choix est appliqué immédiatement à la sélection — pas de bouton « Appliquer »
  (les politiques par défaut sont réversibles ; le Journal reste la preuve).
  « Rejeter » vs « Refuser » est accompagné d'une infobulle explicative.
- **Carte Journal** : niveau de journalisation UFW (désactivé → complet) en
  liste déroulante + explication d'une ligne.
- **Table Règles** : en-têtes triables, zébrage discret, colonne action colorée
  (vert Autoriser / rouge Bloquer / ambre Rejeter), IPv6 : badge « v6 » sur la
  règle logique au lieu de deux lignes dupliquées (une règle = une intention ;
  l'app dissimule la complexité mais reste fidèle à la réalité).
  Recherche instantanée au-dessus de la table.
- Les boutons d'action rares (Rapport, Réinitialiser) sont *en bas* : le regard de
  l'expert descend vers le rare, jamais l'inverse. Réinitialiser demande la saisie
  de « RÉINITIALISER » (irréversible).

### 4.4 Modale « Ajouter une règle »

```
┌ Ajouter une règle ────────────────────────────────┐
│  [ Autoriser │ Bloquer │ Rejeter ]  (segmenté)   │
│  [ Entrée │ Sortie ]              (segmenté)     │
│                                                   │
│  Port / service   [ 8080        ]  ✓              │
│  Protocole        [ TCP │ UDP │ les deux ]        │
│                                                   │
│  ▸ Options avancées                               │
│  ┌───────────────────────────────────────┐       │
│  │ Source      [ Partout ou 192.168.0.0/24 ]      │
│  │ Destination [ Partout ou 10.0.0.5       ]     │
│  │ Position    [ À la fin / n° 1…9         ]     │
│  │ Commentaire [ Serveur web interne       ]      │
│  │ Profil     [ …liste ufw app list…      ]      │
│  └───────────────────────────────────────┘       │
│                                                   │
│  Aperçu : ufw allow in proto tcp from any \       │
│           to any port 8080 comment '…'   (mono)  │
│                                                   │
│                       [ Annuler ]  [ Ajouter ]    │
└──────────────────────────────────────────────────┘
```

Point clé : un **aperçu de la commande réelle** en bas de la modale, en police
mono. L'expert lit la commande ; le débutant l'ignore. Cette transparence
« à double lecture » est le fil conducteur de l'app : chaque écran complexe
offre sa contrepartie textuelle exacte.

### 4.5 Écran « Fail2Ban »

```
┌ Fail2Ban ────────────────────────────────────────────────────────┐
│  ┌ Service ──────────┐ ┌ Prisons ────┐ ┌ Bannis actuellement ──┐ │
│  │ ● Actif (1.1.1)    │ │ 3 prisons   │ │ 12 IP · 2 prisons    │ │
│  │ [Démarrer/Arrêter] │ │ actives     │ │                      │ │
│  │ [Redémarrer] [Rech.]│ └─────────────┘ └──────────────────────┘ │
│                                                                  │
│  ┌ Prisons ───────────────────────────────────────────────────┐ │
│  │ Prison  Bannis  Échecs  bantime  findtime  maxretry        │ │
│  │ sshd       2      14     10 min    10 min    5      [Ouvrir]│ │
│  │ nginx      0       0     1 h       10 min    3      [Ouvrir]│ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌ Prison « sshd » ───────────────────────────────────────────┐ │
│  │ Échecs actuels : 3 · Journal : /var/log/auth.log (mono)     │ │
│  │ Bannir une IP [ 203.0.113.40 ] [Bannir]                    │ │
│  │ ┌ IP bannies ────────────────────────────────────────┐    │ │
│  │ │ 203.0.113.40   depuis 14 min   [Débannir]           │    │ │
│  │ │ 198.51.100.7   depuis  2 min   [Débannir]           │    │ │
│  │ └─────────────────────────────────────────────────────┘    │ │
│  │ [ Recharger la prison ] [ Journal du service ]              │ │
│  └────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

- Même grammaire visuelle que l'écran Pare-feu (3 cartes + table) : la
  reconnaissance bat la variété.
- Bannir/débannir : actions rapides, toast + Annuler. Le champ IP valide la
  syntaxe (IPv4/IPv6/CIDR) et désactive le bouton sinon.
- Le bouton « Journal du service » ouvre un panneau de lecture des derniers
  journaux systemd (`journalctl -u fail2ban`), en mono, lecture seule.

### 4.6 Écran « Journal »

Table horodatée de toutes les commandes exécutées par l'application :
heure · domaine · opération · commande (mono) · résultat (code + badge).
Filtres par domaine et par statut. Copie en un clic. Plafonné à 500 entrées
en mémoire (anti-croissance), mention affichée.

### 4.7 Écran « Paramètres »

- Thème : Sombre (défaut) / Clair. Taille de texte : S / M / L.
- Actualisation automatique : activée, intervalle (3–60 s).
- Comportement SSH : préfixe sudo (jamais de mot de passe stocké).
- Emplacement des fichiers de configuration + bouton « Révéler ».

### 4.8 Composants transverses

- **Toast** (bas-droite, 4 s, empilable) : succès / erreur / info ;
  optionnellement un bouton d'action (« Annuler »). Les erreurs restent
  jusqu'à fermeture manuelle (jamais de message d'erreur éphémère).
- **Modale** : le fond est assombri ; le focus clavier est piégé ;
  Échap = Annuler ; Entrée = action principale. Le bouton dangereux est
  toujours le plus à droite, en rouge plein.
- **Indicateur de connexion** : point vert/bleu/ambre/rouge + libellé, jamais
  de vert si une commande vient d'échouer (l'état est re-évalué à chaque cycle).
- **Chargement** : le bouton devient spinner + texte « … » ; les données
  précédentes restent affichées, grisées à 60 % — jamais d'écran vide qui flash.

---

## 5. Système de design

### 5.1 Palette (thème sombre — défaut)

| Rôle | Valeur | Usage |
|---|---|---|
| Fond page | `#15161A` | arrière-plan |
| Fond carte | `#1E2026` | cartes, tables |
| Bordure | `#2A2D35` | séparateurs discrets |
| Texte 1er | `#E8EAED` | titres, valeurs |
| Texte 2e | `#9AA0AA` | libellés, aide |
| Accent | `#4C8DFF` | sélection, boutons principaux, liens |
| Succès | `#3FB96F` | actif, autorisé |
| Avertissement | `#E5A13C` | attention, rejeter |
| Danger | `#E5484D` | désactivé, bloquer, destructif |

Thème clair dérivé (fond `#F7F8FA`, cartes `#FFFFFF`, accent `#2F6FE4`,
mêmes sémantiques). Le rouge n'est **jamais** utilisé hors d'un sens danger.

### 5.2 Typographie

- **IBM Plex Sans** (embarquée) — Regular 14 px, Medium 14–15 px, SemiBold 16–20 px
  pour les titres de cartes, Bold réservé aux titres d'écran.
- **IBM Plex Mono** — 13 px — réservée aux *données techniques* : adresses IP,
  CIDR, ports, commandes, empreintes SSH, sorties brutes. La mono rend les
  données comparables d'une ligne à l'autre et signale « ceci est exactement
  ce qui sera exécuté ».
- Hiérarchie : écran 20 px SemiBold → carte 12 px majuscules espacées →
  valeur 15–18 px → aide 12,5 px texte 2e.

### 5.3 Espacement & formes

- Grille de 8 px (marges de cartes 16 px, gouttières 12 px, padding de
  cellule 8×6). Coins arrondis 12 px (cartes), 8 px (boutons/champs).
- Densité confortable, pas « dashboardsque » : les tables ont ≥ 32 px de
  hauteur de ligne, cible de clic ≥ 28 px.

### 5.4 Icônes

Jeu **Material Icons** (embarqué) en 18/20 px, unicolore, héritant la couleur
du texte. Une icône = toujours accompagnée d'un libellé texte (jamais
d'icône seule pour une action). Pas d'emoji.

---

## 6. Micro-interactions et réactivité

| Situation | Comportement |
|---|---|
| Clic sur action | bouton verrouillé immédiatement (anti double-clic), spinner, puis résultat |
| Rafraîchissement auto | silencieux, indicateur « il y a Ns » ; suspendu si fenêtre non focalisée |
| Erreur de commande | toast rouge persistant + entrée Journal ; le message système est affiché tel quel |
| Succès | toast vert 4 s (désactivable dans Paramètres) |
| Action réversible | toast avec « Annuler » (débannir/bannir, supprimer règle → recréable via Journal) |
| Déconnexion réseau | état figé gris + bandeau reconnexion ; saisies en cours préservées |

---

## 7. Accessibilité et clavier

- Navigation clavier intégrale ; focus visible (contour accent 2 px).
- Raccourcis : `Ctrl+R` actualiser · `Ctrl+N` nouvelle règle · `Ctrl+1…5`
  onglets · `Échap` fermer modale/toast d'erreur.
- Taille de texte réglable (S/M/L → facteur 0.9/1.0/1.15) — tout le layout
  s'adapte, les tables deviennent défilantes.
- Contraste minimum AA (4.5:1) vérifié sur les deux thèmes pour les textes.
- Les couleurs d'état sont *toujours* doublées d'un libellé textuel
  (accessible aux daltoniens et en capture d'écran N&B).

---

## 8. La sécurité rendue visible (UX de confiance)

| Mécanisme | Traduction interface |
|---|---|
| Escalade root unique par session (polkit via `pkexec`, helper local à liste blanche stricte : seuls `ufw`, `fail2ban-client`, `systemctl`, `journalctl` sur des unités autorisées) | 1 boîte polkit à la première action, puis rien ; explication claire au premier lancement |
| Helper root exposé sur socket UNIX protégée par identifiant utilisateur (`SO_PEERCRED`) + arrêt automatique après inactivité | invisible (par conception), documenté dans Paramètres > À propos |
| SSH : TOFU — empreinte SHA256 affichée en mono à la 1re connexion, mémorisée, avertissement fort si changement | boîte « Clé du serveur inconnue — vérifiez auprès de votre hébergeur » |
| Mots de passe (session et sudo) jamais écrits sur le disque ; effacés de la mémoire dès l'usage | mention permanente sur l'écran Connexion |
| Commande exacte envoyée au système | aperçu mono dans les modales + Journal complet |

---

## 9. Matrice des fonctionnalités v1

| | v1 | Futur |
|---|---|---|
| UFW : état, activer/désactiver, reload | ✅ | |
| Politiques par défaut (entrée/sortie/routage) | ✅ | |
| Journalisation (6 niveaux) | ✅ | courbe de blocage |
| Règles : liste, recherche, ajout (base+avancé), suppression | ✅ | réordonnancement glisser |
| Profils d'application, rapport, réinitialisation | ✅ | import/export de règles |
| Fail2Ban : état, prisons, bannis, ban/unban, paramètres prison | ✅ | édition de jail.local |
| Service fail2ban : démarrer/arrêter/redémarrer, recharger | ✅ | |
| Journaux système (fail2ban, blocs UFW) | ✅ | filtres avancés |
| Local root/polkit, SSH (mot de passe/clé, TOFU, sudo optionnel) | ✅ | agent, clés générées |
| Journal des actions de l'app, paramètres, thèmes | ✅ | i18n EN/ES |

## 10. Décisions refusées (sobriété assumée)

- **Pas d'emoji** ni d'illustrations : le contenu réel est l'interface.
- **Pas d'animations décoratives** ; seuls les changements d'état sont visibles.
- **Pas de mode « sécurité en 1 clic »** qui masquerait les règles réelles.
- **Pas d'édition de fichiers de configuration par formulaire** (jail.local, ufw
  user rules) en v1 : trop de variations de format = risque de casser la config
  d'un utilisateur ; l'app gère par les commandes officielles uniquement.
- **Pas de persistance des mots de passe**, même chiffrée : le risque n'est pas
  compensé par le confort.
