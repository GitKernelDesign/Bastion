# Bastion — Étude ergonomique de l'interface

> Interface graphique simple, sobre, élégante et conviviale pour gérer intégralement
> **UFW** et **Fail2Ban**, en local ou à distance via SSH.

---

## 1. Contexte et objectifs

### 1.1 Ce que l'application doit permettre

| Domaine | Besoin utilisateur |
|---|---|
| **UFW** | Connaître l'état du pare-feu d'un coup d'œil ; l'activer/désactiver ; régler les politiques par défaut ; ajouter/supprimer des règles (port, protocole, IP source/destination, profil d'application, commentaire) ; régler la journalisation ; voir le rapport ; réinitialiser. |
| **Fail2Ban** | Savoir si le service tourne ; lister les prisons (jails) ; voir les IP bannies ; bannir / débannir une IP ; consulter les paramètres d'une prison (bantime, findtime, maxretry) ; démarrer/arrêter/redémarrer le service ; recharger une prison ; consulter les journaux système. |
| **ClamAV** | Voir l'état des services `clamav-daemon` et `clamav-freshclam` ; connaître la version du moteur, le nombre de signatures et leur date ; lancer une mise à jour ; analyser un chemin à la demande (lecture seule) ; consulter les journaux `clamd` et `freshclam`. |
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

**Fenêtre large (≥ 720 px) — barre latérale + barre d'état**

```
Pare-feu                                        ← écran courant
├─ Barre supérieure (54)   ● Local (assistant root) ›     Auto  ⟳  ⋮
├─ Barre latérale (224)    identité · Administration · Suivi · Paramètres
│                          pied : v0.3.0 · ● ● · Local (assistant root)
├─ En-tête (fixe)          Pare-feu
│                          État du pare-feu UFW, politiques et règles en vigueur.
├─ Contenu (défile)
│  ├─ Carte « État du pare-feu »      interrupteur + synthèse + actions
│  ├─ Tuiles : 7 Règles · 5 Autorisations · 1 Blocages
│  ├─ Carte « Politiques par défaut »
│  ├─ Carte « Règles (7) »
│  └─ Carte « Détails techniques » (repliée)
└─ Barre d'état (26)       Pare-feu activé                              Auto
```

**Fenêtre compacte (< 720 px) — navigation basse**

```
Pare-feu
├─ Barre supérieure (54)   ● Local (assistant root) ›           ⟳   ⋮
├─ En-tête (fixe)          Pare-feu + sous-titre
├─ Contenu (défile, une colonne)
│  ├─ Carte pleine largeur
│  └─ Tuiles empilées (une colonne si la largeur ne permet pas deux tuiles)
├─ Barre d'état (26)       Pare-feu activé                             Auto
└─ Navigation basse (62)   Connexion · Pare-feu · Fail2Ban · Antivirus · Journal · Paramètres
```

- **Barre latérale** (224 px) : identité (« Bastion · UFW & Fail2Ban »), deux
  groupes — *Administration* (Connexion, Pare-feu, Fail2Ban, Antivirus) et *Suivi*
  (Journal) —, Paramètres ancré en bas, puis un pied de barre : version, deux points
  d'état (outils détectés) et la cible courante. L'entrée active est une pilule
  teintée doublée d'une barre d'accent à gauche ; les entrées concernées portent un
  badge chiffré (règles, prisons), jamais un « 0 ».
- **Pas de repli en icônes** : sous 720 px la navigation passe en bas d'écran. Un
  seul point de bascule, moins de modes à maintenir, des libellés toujours lisibles.
- **Barre supérieure** (54 px) : elle donne le **contexte** (quelle machine suis-je
  en train de toucher ?) — pastille verte (local), bleue (SSH, nom du serveur en
  mono), rouge (déconnecté) —, puis actualiser et menu. C'est la seule barre toujours
  identique d'un écran à l'autre.
- **En-tête d'écran** : titre 21 px SemiBold + sous-titre 12 px en texte secondaire ;
  il reste fixe pendant que le contenu défile en dessous.
- **Cartes** : rayon 13 px, bordure 1 px, padding 16–18 px, **aucune ombre** — la
  profondeur vient de trois couches (chrome, fond de travail, carte).
- **Barre d'état** (26 px) : dernier événement à gauche, état de l'actualisation
  automatique à droite (« Auto · 5 s », en vert quand elle est active ; raccourcis
  en infobulle). Les notifications se posent **au-dessus** d'elle (voir 4.8).

### 4.2 Écran « Connexion »

```
Connexion
├─ Carte « Cette machine »                                    badge LOCAL
│  ├─ pastilles d'outils : ✓ ufw 0.36.2   ✓ Fail2Ban v1.1.1
│  ├─ [ Se connecter ]                              (pleine largeur, principal)
│  └─ « Une fenêtre de mot de passe apparaîtra une seule fois pour la session. »
└─ Carte « À distance (SSH) »                                  badge SSH
   ├─ Profil enregistré   [ … v ]                                    🗑
   ├─ Hôte                [ … ]
   ├─ Port [ 22 ]   Utilisateur [ … ]          (une seule ligne)
   ├─ Authentification   [ Mot de passe │ Clé privée ]   (segmenté, pleine largeur)
   ├─ Mot de passe (ou clé + phrase secrète)   [ … ]  👁
   ├─ [ Se connecter ]                              (avant les options repliées)
   ├─ « L'empreinte de la clé du serveur sera vérifiée à la première connexion. »
   └─ ▾ Options de connexion : exécution via sudo (+ mot de passe), enregistrer le profil
```

- Chaque carte porte son badge (`LOCAL` / `SSH`) et une icône dans une pastille :
  on reconnaît la carte avant de lire son titre.
- Carte locale : l'app détecte UFW et Fail2Ban **avant** la connexion (pastilles
  vertes/rouges, nom d'outil dédoublonné de sa version) → l'absence d'outil se voit
  immédiatement, sans droits root.
- Carte SSH : profils enregistrés en premier (liste déroulante + suppression),
  hôte, port et utilisateur sur une ligne, puis l'**authentification** en sélecteur
  segmenté pleine largeur (mot de passe / clé privée + phrase secrète).
- Le bouton **Se connecter** est placé *avant* les options repliées : le parcours
  courant (hôte, utilisateur, mot de passe, connexion) ne demande aucun dépliage.
- « Options de connexion » replie le rare : exécution via `sudo` (et son mot de
  passe), enregistrement du profil sous un nom.
- Le champ d'empreinte de clé, lui, reste toujours visible : c'est la promesse de
  sécurité (TOFU) que l'on veut lire avant de cliquer.

### 4.3 Écran « Pare-feu » (UFW)

```
Pare-feu
├─ [Bandeau] ⛔ Pare-feu inactif — votre machine est exposée.  [ Activer maintenant ]
│                                                       (affiché seulement si arrêté)
├─ Carte « État du pare-feu »                                        il y a 4 s
│  ├─ ● Actif                                              [interrupteur]
│  ├─ Service UFW · active                                 ufw 0.36.2
│  ├─ Entrée Refuser · Sortie Autoriser · Routage Refuser · Journalisation Faible
│  └─ [Rapport]  [Recharger]        Réinitialiser…            (lien discret)
├─ Tuiles : 7 Règles · 5 Autorisations · 1 Blocages
├─ Carte « Politiques par défaut »
│  ├─ Entrée   [Autoriser │ Refuser │ Rejeter]        Journalisation [Faible v]
│  ├─ Sortie   [Autoriser │ Refuser │ Rejeter]        (2 colonnes en large)
│  ├─ Routage  [Autoriser │ Refuser │ Rejeter]        (empilé en compact)
│  └─ Rejeter = refuse avec notification ; Refuser = silencieux.
├─ Carte « Règles (7) »                                       [+ Ajouter]  ⟳
│  ├─ ⌕ Rechercher une règle
│  └─ # · Action · De · Vers · Port · Commentaire
└─ Carte « Détails techniques » (repliée) : sortie brute, service systemd
```

- **Un seul « héro »** : la carte d'état réunit le point d'état, l'interrupteur
  (44 px de haut, vert/actif), la version, la **synthèse des politiques** en quatre
  pastilles (entrée, sortie, routage, journalisation) et les actions principales.
  La ligne « Service UFW · active » confirme que le service systemd suit l'état.
- **Bandeau d'alerte** uniquement quand le pare-feu est inactif : il explique la
  conséquence et propose l'activation *sur place* (l'utilisateur n'a pas à chercher
  l'interrupteur).
- **Trois tuiles** (règles, autorisations, blocages) : des chiffres qui se lisent
  sans lire. La tuile « journalisation » a été supprimée — l'information figure déjà
  dans le héro.
- **Carte Politiques + journalisation** : une seule carte. En large, deux colonnes
  (politiques à gauche, liste de journalisation à droite) ; en compact, empilées.
  Enregistrement immédiat, pas de bouton « Appliquer » : ces réglages sont
  réversibles et le Journal garde la preuve. Le sens de « Rejeter » vs « Refuser »
  est rappelé en pied de carte.
- **Table des règles** : recherche instantanée, ajout dans un composeur repliable
  (avec aperçu de la commande), colonne action colorée et doublée d'un libellé,
  badge `v6` sur la règle logique plutôt que deux lignes dupliquées.
- **Actions rares et techniques en bas**, dans une carte repliée : rapport UFW,
  sortie brute, service systemd, réinitialisation (saisie de « RÉINITIALISER »).

### 4.4 Modale « Ajouter une règle »

```
Ajouter une règle
├─ [ Autoriser │ Bloquer │ Rejeter ]                        (segmenté)
├─ [ Entrée │ Sortie ]                                      (segmenté)
├─ Port / service    [ 8080 ]  ✓
├─ Protocole         [ TCP │ UDP │ les deux ]
├─ ▸ Options avancées
│  ├─ Source [ Partout ou 192.168.0.0/24 ] · Destination [ Partout ou 10.0.0.5 ]
│  ├─ Position [ À la fin / n° 1…9 ] · Commentaire [ Serveur web interne ]
│  └─ Profil [ … liste ufw app list … ]
├─ Aperçu : ufw allow in proto tcp from any to any port 8080 comment '…'   (mono)
└─ [ Annuler ]   [ Ajouter ]
```

Point clé : un **aperçu de la commande réelle** en bas de la modale, en police
mono. L'expert lit la commande ; le débutant l'ignore. Cette transparence
« à double lecture » est le fil conducteur de l'app : chaque écran complexe
offre sa contrepartie textuelle exacte.

### 4.5 Écran « Fail2Ban »

```
Fail2Ban
├─ Tuiles : 3 Prisons · 5 Bannies · 54 Échecs
├─ Carte « Service Fail2Ban »                                       il y a 4 s
│  ├─ ● Service active           [Redémarrer]  [Arrêter]  [Recharger]
│  └─ Journal · Toutes les IP                              (liens à droite)
├─ Maître-détail au-delà de 760 px, empilé en dessous
│  ├─ Carte « Prisons (3) »                                     40 %
│  │  ├─ ⌕ Filtrer les prisons
│  │  └─ Prison · Bannies · Échecs                (chiffres alignés à droite)
│  │     sshd 3 42 ›   ·   nginx-http-auth 2 12 ›   ·   postfix-sasl 0 0 ›
│  └─ Carte « Prison « sshd » »                                 60 %      ⟳
│     ├─ Échecs 42 · Total 1 284
│     ├─ maxretry 5 · bantime 1 h · findtime 10m
│     ├─ Surveille : /var/log/auth.log
│     ├─ IP [ 203.0.113.40 ]  (Bannir)              (validée IPv4/IPv6/CIDR)
│     └─ IP BANNIES (3)     203.0.113.42 · depuis 14 min          ⧉
```

- **Maître-détail** au-delà de 760 px : la liste des prisons occupe 40 %, le
  détail 60 % ; en dessous, les deux cartes s'empilent. On garde le contexte de la
  liste en lisant le détail — plus besoin de revenir en arrière.
- Les **colonnes chiffrées sont alignées à droite** (en-têtes compris) et rendues
  par `paint` : les nombres se comparent verticalement, comme dans un tableau
  comptable.
- La liste est filtrable par nom, et la prison sélectionnée est marquée (fond teinté
  + chevron). La flèche indique qu'une ligne est cliquable, pas un bouton.
- Le détail concentre : compteurs, paramètres de la prison, fichier surveillé,
  champ de bannissement (validé : IPv4/IPv6/CIDR) et liste des IP bannies avec copie
  par ligne.
- Les journaux restent à portée de main dans l'en-tête du service (journal du
  service, toutes les IP bannies) : ce sont des consultations, pas des actions.

### 4.6 Écran « Journal »

```
Journal
├─ Sous-titre : chaque commande passée au système, avec son résultat.
└─ Carte
   ├─ Filtres comptés : (Tout 10) (Erreurs 1) (Pare-feu 4) (Fail2Ban 5)
   │                                          [⧉ Copier la vue]     🗑
   ├─ En-tête : Heure · État · Commande · Détail
   └─ Colonnes fixes, troncature « … » + infobulle, copie par ligne (⧉)
      08:23:08  ✓  clamscan --version                          code 0
      08:22:31  ✗  ufw delete 5                   ERROR: Could not del…
      08:21:54  ✓  fail2ban-client set sshd unbanip 203.0.113.42
```

- Table horodatée de **toutes** les commandes exécutées : heure, état, commande
  (mono), détail. Une ligne = une commande réelle, jamais une reformulation.
- **Colonnes à largeur fixe** : la commande et le détail sont tronqués avec « … »
  au lieu de forcer un retour à la ligne par mot (l'ancienne version cassait la
  lecture verticale). L'infobulle donne le texte complet, la copie par ligne (⧉)
  le met dans le presse-papiers.
- Filtres en pastilles **comptées** (tout, erreurs, pare-feu, Fail2Ban) et « Copier
  la vue » pour coller un rapport dans un ticket.
- Effacement confirmé par une modale ; plafond de 500 entrées en mémoire
  (anti-croissance), mention affichée.

### 4.7 Écran « Paramètres »

Quatre cartes, sans bouton « Enregistrer » : chaque contrôle est persisté
immédiatement.

```
Paramètres
├─ Carte « Apparence »
│  ├─ Langue    [ Français v ]
│  ├─ Thème     [ Système │ Clair │ Sombre ]
│  ├─ Texte     [ A− │ A │ A+ ]
│  └─ Notifications de succès (4 s)                          [switch]
├─ Carte « Actualisation automatique »
│  ├─ Actualiser l'état périodique                           [switch]
│  ├─ Intervalle    [——●————————] 5.0 s              (toujours actif)
│  └─ Raccourcis clavier : Ctrl+1…6 · Ctrl+R · Ctrl+N
├─ Carte « Données et sécurité »   chemin (mono) · [Ouvrir] [Oublier]
└─ Carte « À propos »              ⬢ Bastion v0.3.0 · GPL-3.0
```

- **Apparence** : langue (nom natif, sélectionnable au clavier), thème
  **Système / Clair / Sombre** (le système par défaut : l'app suit le bureau),
  taille de texte (A− / A / A+), notifications de succès.
- **Actualisation automatique** : l'interrupteur commande l'actualisation
  silencieuse ; le curseur d'intervalle reste **toujours actif** (le griser serait
  trompeur et plus difficile à viser). Les raccourcis clavier sont rappelés en bas
  de carte.
- **Données et sécurité** : chemin de configuration (mono, tronqué au milieu avec
  infobulle), « Ouvrir le dossier », « Oublier les hôtes connus » (TOFU) et compteur.
- **À propos** : version, licence, une phrase sur l'assistant root.
- En large, les quatre cartes forment une grille ; en compact, une seule colonne.

### 4.8 Composants transverses

- **Toast** (au-dessus de la barre d'état, 4 s, empilable, 4 au plus) : succès /
  erreur / info ; optionnellement un bouton d'action (« Annuler »). Les erreurs
  restent jusqu'à fermeture manuelle. Le placement est calculé pour **ne jamais
  recouvrir** la barre d'état ni la navigation basse.
- **Modale** : fond assombri, focus piégé, `Échap` = Annuler, `Entrée` = action
  principale. Le bouton dangereux est le plus à droite, en teinte danger pleine.
- **Bouton désactivé** : fond neutre (`card_hover` atténué) + contour, texte en
  secondaire — jamais une barre d'accent délavée qui ressemblerait à une barre de
  progression.
- **Indicateur de connexion** : point vert/bleu/ambre/rouge + libellé, jamais de
  vert si la dernière commande a échoué (l'état est réévalué à chaque cycle).
- **Chargement** : le bouton devient spinner + texte ; les données précédentes
  restent affichées, atténuées — jamais d'écran vide qui clignote.
- **Tuiles** : grille qui évite les lignes incomplètes (quatre tuiles = 2 × 2, pas
  3 + 1) et conserve la largeur des tuiles sur la dernière ligne.
- **Champs** : libellé au-dessus, hauteur 34 px, rayon 8 px, placeholder en
  secondaire ; les champs de mot de passe ont un bouton œil.

---

## 5. Système de design

### 5.1 Palette

Trois couches par thème : **chrome** (barres) → **fond de travail** → **cartes**.
La hiérarchie se lit sans trait ; les bordures ne servent qu'aux champs et aux
séparateurs.

| Rôle | Clair | Sombre | Usage |
|---|---|---|---|
| Chrome (`bg_alt`) | `#FAF7F4` | `#0E0B0A` | barre latérale, barres sup./état |
| Fond (`bg`) | `#F2EDE9` | `#141110` | zone de travail |
| Carte (`card`) | `#FFFFFF` | `#1E1917` | cartes, tableaux |
| Champ (`field`) | `#FAF7F4` | `#171312` | saisies, aperçus mono |
| Bordure | `#E6DBD3` | `#352C28` | séparateurs, contours |
| Texte | `#1F1A17` | `#F5EFEB` | titres, valeurs |
| Texte 2d | `#6F625A` | `#A99A90` | libellés, aides |
| Accent | `#B4532F` | `#E38B66` | boutons principaux, sélection, liens |
| Succès | `#0F7B57` | `#57D9A3` | actif, autorisé |
| Avertissement | `#A15C07` | `#F5C264` | attention, « rejeter » |
| Danger | `#B4233C` | `#FF8095` | inactif, bloqué, destructif |

Le neutre est **chaud** (papier, encre) et l'accent est le terracotta de l'icône :
l'application n'arbore pas le bleu « console » habituel. Le texte posé sur un aplat
d'accent est choisi automatiquement (blanc sur terracotta sombre, encre sur
terracotta clair) pour dépasser 4,5:1 ; un test unitaire vérifie chaque couple.

### 5.2 Typographie

- **IBM Plex Sans** (embarquée) — corps 14 px, libellés 13 px, titres de cartes
  14,5 px Medium, titres d'écran 21 px SemiBold, aides 11–12 px.
- **IBM Plex Mono** — 12,5 px — réservée aux *données techniques* : adresses IP,
  CIDR, ports, commandes, empreintes SSH, sorties brutes. La mono rend les données
  comparables d'une ligne à l'autre et signale « ceci est exactement ce qui sera
  exécuté ».
- Une seule échelle (`DISPLAY` 21 · `TITLE` 14,5 · `BODY` 14 · `LABEL` 13 ·
  `SMALL` 12 · `MICRO` 11) multipliée par le facteur d'accessibilité
  (A− 0,9 · A 1 · A+ 1,15).

### 5.3 Espacement & formes

- Échelle de 4 px (`XS` 4 · `S` 8 · `M` 12 · `L` 16) ; marges de page 24 px en
  large, 14 px en compact ; gouttière de grille 12 px.
- Rayons : cartes 13 px, boutons 9 px, champs 8 px, modales 16 px.
- Hauteur de bouton 34 px, ligne de tableau ≥ 34 px, cible de clic ≥ 28 px : la
  densité reste confortable au doigt comme à la souris.
- **Aucune ombre** : une carte se distingue par sa couleur et sa bordure.

### 5.4 Icônes

Jeu **Material Icons** (embarqué) en 18/20 px, unicolore, héritant la couleur
du texte. Une icône = toujours accompagnée d'un libellé texte (jamais
d'icône seule pour une action). Pas d'emoji.

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
- Raccourcis : `Ctrl+R` actualiser · `Ctrl+N` nouvelle règle · `Ctrl+1…6`
  écrans · `Échap` fermer modale/toast d'erreur.
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
| ClamAV : services, bases de signatures, mise à jour, analyse à la demande | ✅ | quarantaine |
| Journal des actions de l'app, paramètres, thèmes système/clair/sombre | ✅ | filtre texte |
| Interface en 8 langues (FR/EN/ES/DE/PT/IT/UK/RU) | ✅ | |

## 10. Décisions refusées (sobriété assumée)

- **Pas d'emoji** ni d'illustrations : le contenu réel est l'interface.
- **Pas d'animations décoratives** ; seuls les changements d'état sont visibles.
- **Pas de mode « sécurité en 1 clic »** qui masquerait les règles réelles.
- **Pas d'édition de fichiers de configuration par formulaire** (jail.local, ufw
  user rules) en v1 : trop de variations de format = risque de casser la config
  d'un utilisateur ; l'app gère par les commandes officielles uniquement.
- **Pas de persistance des mots de passe**, même chiffrée : le risque n'est pas
  compensé par le confort.

---

## 11. Refonte de l'interface (v0.3.0)

L'intention d'origine est conservée — même sobriété, même transparence, mêmes
parcours. La refonte a porté sur la **lisibilité**, la **densité** et la
**cohérence des contrôles**, écran par écran.

| Avant | Après | Pourquoi |
|---|---|---|
| 5 entrées de navigation | 6 entrées, groupées *Administration* / *Suivi*, badges chiffrés | Antivirus devient un écran de plein droit ; le pied de barre indique les outils détectés |
| Titre d'écran + pastille de fraîcheur globale | Titre + sous-titre ; fraîcheur portée par **chaque carte** | Une seule horloge par donnée : plus de doublon entre l'en-tête et les cartes |
| Pare-feu : 4 cartes + 4 chiffres | **Héro unique** (état, interrupteur, synthèse, actions), 3 tuiles, politiques + journalisation réunies | Le regard fait un seul arrêt avant d'agir ; moins de cartes à parcourir |
| Boutons désactivés en aplat d'accent délavé | Fond neutre + contour, texte secondaire | Un bouton grisé ressemblait à une barre de progression ; l'état devient sans ambiguïté |
| Interrupteurs tous affichés en position ON | Interrupteur animé par l'identifiant de **son** interaction | Bug egui : `ui.scope_id()` est partagé par les sous-`Ui` frères → animation et clic sur la même valeur |
| Sélecteur segmenté empilé verticalement | Toujours horizontal, réparti sur la largeur pour l'authentification | Un sélecteur à deux choix ne doit jamais ressembler à un formulaire |
| Fail2Ban : table puis détail empilés | **Maître-détail** 40/60, colonnes numériques alignées à droite, filtre | Lire une prison sans perdre la liste de vue |
| ClamAV : deux cartes verticales, fraîcheur détachée | Grille services / bases, fraîcheur dans l'en-tête, actions homogènes | Même grammaire que les autres écrans |
| Journal : texte enroulé, détail noyé | Colonnes fixes, détail = code/erreur, troncature + infobulle, copie par ligne | La lecture verticale des commandes redevient possible |
| Réglages : thème sombre/clair, langue en menu déroulant simple | **Système** / clair / sombre, langue en liste native, raccourcis rappelés | Suivre le bureau par défaut ; 8 langues doivent rester trouvables |
| Toasts recouvrant la barre d'état | Toasts ancrés **au-dessus** | Une notification ne doit jamais masquer l'état de l'application |
| Tuiles 3 + 1 sur la dernière ligne | Colonnes complètes (4 → 2 × 2), largeurs conservées | Une grille irrégulière se lit comme une erreur |

**Décisions refusées pendant la refonte** : pas de barre latérale repliable (un seul
point de bascule), pas d'onglets internes aux écrans (le défilement suffit), pas de
graphiques ni de jauges (des chiffres et des états suffisent), pas de densité
« dashboard » (les lignes respirent à 34 px).

**Vérification** : 13 captures hors écran (clair/sombre, compact/large, FR/EN/DE)
produites par un test ignoré, contrastes AA testés unitairement, 48 tests verts,
`cargo check` sans avertissement.
