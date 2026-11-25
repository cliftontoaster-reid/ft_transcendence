# Suivi des modules — Dionysus (ft_transcendence)

Objectif : **14 points minimum** (Major = 2 pts, Minor = 1 pt).
Ne cocher un module que lorsqu'il est **100 % fonctionnel** (tout module incomplet vaut **0 pt**).

## 1. Catégorie Web

- [ ] **Framework frontend + backend** *(Major — 2 pts)* — SvelteKit + backend structuré *(ou Minor — 1 pt si seul le frontend utilise un framework)*
  - [ ] Architecture front et back opérationnelle sans erreur console
- [ ] **Real-time features via WebSockets** *(Major — 2 pts)*
  - [ ] Chat temps réel et synchronisation de lecture (play / pause / seek)
  - [ ] Gestion propre des connexions, déconnexions et reconnexions
- [ ] **Allow users to interact with other users** *(Major — 2 pts)*
  - [ ] Chat de base (envoi / réception de messages)
  - [ ] Système de profil utilisateur
  - [ ] Système d'amis (ajout / retrait / liste)
- [ ] **Real-time collaborative features** *(Minor — 1 pt)*
  - [ ] Salon de visionnage synchronisé ("watch party") entre amis
- [ ] **File upload and management system** *(Minor — 1 pt)*
  - [ ] Upload vidéos / avatars avec validation front & back (type, taille, format)
  - [ ] Stockage sécurisé, barre de progression, aperçu et suppression
- [ ] **Use an ORM for the database** *(Minor — 1 pt)*
  - [ ] ORM utilisé côté backend pour les requêtes base de données
- [ ] **Advanced search functionality** *(Minor — 1 pt)*
  - [ ] Recherche de vidéos avec filtres, tri et pagination

## 2. Catégorie User Management

- [ ] **Standard user management and authentication** *(Major — 2 pts)*
  - [ ] Édition du profil et avatar (avec avatar par défaut)
  - [ ] Liste d'amis avec statut en ligne et page profil dédiée
- [ ] **Remote authentication with OAuth 2.0** *(Minor — 1 pt)*
  - [ ] Connexion via fournisseur externe (42, Google, GitHub, etc.)
- [ ] **Two-Factor Authentication (2FA)** *(Minor — 1 pt)*
  - [ ] Activation, vérification et gestion de la 2FA

## 3. Catégorie Accessibility and Internationalization

- [ ] **Multiple languages (i18n)** *(Minor — 1 pt)*
  - [ ] Au moins 3 langues complètes, sélecteur UI et textes traduits
- [ ] **Support for additional browsers** *(Minor — 1 pt)*
  - [ ] Testé et validé sur Chrome + 2 autres navigateurs (ex. Firefox, Edge/Safari)
  - [ ] Limites éventuelles documentées dans le `README.md`

## 4. Catégorie Data and Analytics & DevOps (optionnel selon l'équipe)

- [ ] **GDPR compliance features** *(Minor — 1 pt)* — Export des données et suppression de compte avec confirmation
- [ ] **Data export and import** *(Minor — 1 pt)* — Export / import de données (JSON / CSV)
- [ ] **Health check and status page + automated backups** *(Minor — 1 pt)*
- [ ] **Monitoring system (Prometheus + Grafana)** *(Major — 2 pts)*

## 5. Prérequis obligatoires (Chapitre III — non négociables)

- [ ] Déploiement en un clic via conteneurisation (Docker / Podman)
- [ ] `.env` ignoré par Git + `.env.example` fourni
- [ ] Authentification email / mot de passe sécurisée (hash + salt) et validation des entrées front & back
- [ ] Connexions vers le backend en HTTPS
- [ ] Pages **Privacy Policy** et **Terms of Service** accessibles et complètes (pas des placeholders)
- [ ] Aucune erreur ou warning JS dans la console Chrome
- [ ] `README.md` conforme (1re ligne italique avec les logins 42, Description, Instructions, Resources + usage de l'IA)