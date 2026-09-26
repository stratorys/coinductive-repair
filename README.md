# Rust → JSON → CUE

Exemple exécutable de validation d'une API Rust et de son comportement observé.

## Fonctionnement de bout en bout

Les fichiers `.cue` sont les spécifications : ils décrivent ce que le code doit
faire. Les fichiers `.json` sont des observations : ils décrivent ce que le code
expose ou ce qu'il a réellement produit. `cue vet` compare les deux.

`scripts/check.sh` exécute ces étapes dans l'ordre :

1. Rustdoc analyse la bibliothèque et écrit `target/doc/coinductive_repair.json`.
   Ce JSON décrit l'API ; il ne contient pas les corps des fonctions.
2. `uv run` exécute l'adaptateur qui extrait la signature de `add` dans
   `contracts/api.json`. CUE la compare à `specs/api.cue`.
3. `cargo run` appelle réellement `add` avec 260 couples d'entrées et écrit les
   résultats dans `contracts/observations.json`. CUE les compare aux équations
   de `specs/behavior.cue`.
4. `cargo run --example account` exécute onze opérations bancaires successives
   et écrit `contracts/account.json`. CUE compare les soldes et statuts à
   `specs/account.cue`, puis vérifie que les transitions s'enchaînent.
5. `uv run` exécute le test qui corrompt sept copies temporaires de cette trace.
   Chaque copie doit être refusée par CUE. Les copies sont ensuite supprimées.

Un rejet d'opération bancaire est un résultat attendu, par exemple lorsqu'un
retrait manque de fonds. Un rejet par CUE signifie que le JSON contredit la
spécification. Les lignes « Rejet confirmé » indiquent que les erreurs injectées
ont bien été détectées, pas que la trace originale contient des erreurs.

Les trois JSON de `contracts/` sont versionnés pour pouvoir lire et comparer les
observations dans Git. Le script les régénère à chaque exécution. Les modifier
manuellement ne corrige pas Rust : il faut modifier le code et relancer le script.
Le JSON rustdoc brut reste dans `target/`, qui est ignoré par Git.

## Exécution

Prérequis : Rust stable, Rust nightly, uv et CUE (module en version 0.17.1).
Les scripts Python sont exécutés exclusivement via `uv run`.

```sh
rustup toolchain install nightly
bash scripts/check.sh
```

Le script régénère trois artefacts versionnés dans `contracts/` :

- `api.json` : fonctions publiques de premier niveau extraites de rustdoc ;
- `observations.json` : entrées et résultats réels de la fonction Rust `add`.
- `account.json` : trace de onze opérations sur un compte bancaire.

Les contrats indépendants sont dans `specs/api.cue` et `specs/behavior.cue`.
Une commande échoue dès qu'une extraction, une exécution ou une validation échoue.

## Ce qui est vérifié

Le contrat d'API exige `pub fn add(u32, u32) -> u32`. Le contrat comportemental
exige que chaque résultat soit égal à la somme des entrées et tienne dans un u32.
Le programme observe les 256 couples de valeurs de 0 à 15, puis quatre cas aux
limites de u32. Les entrées dont la somme dépasse u32::MAX sont hors du domaine
de ce contrat ; la fonction panique dans ce cas, mais cette panique n'est pas
vérifiée par CUE.

Pour observer un échec, modifier `add` pour retourner une valeur erronée puis
relancer le script, ou modifier la signature pour faire échouer le contrat d'API.
Les champs obligatoires et la liste non vide évitent une validation sans données.

## Limites et extension

Rustdoc JSON décrit l'API, pas l'AST des corps de fonctions. L'adaptateur Python
est volontairement limité aux fonctions publiques de premier niveau, non
génériques, synchrones, sûres et à types primitifs. Il ignore les méthodes et
les sous-modules, et refuse les signatures de fonctions extraites qu'il ne sait
pas traduire. Ajouter explicitement leur prise en charge pour élargir l'analyse.

Le format rustdoc JSON est instable : figer une version nightly en CI et adapter
l'extracteur lors d'une mise à jour. Pour les expressions et branches, un autre
extracteur (par exemple basé sur `syn`) est nécessaire.

CUE valide les observations fournies. Cela ne prouve pas la correction pour
toutes les entrées. Étendre le générateur avec des cas métier, des entrées
aléatoires ou des traces de transitions pour vérifier davantage de propriétés.
Les specs doivent exprimer l'intention indépendamment de l'implémentation.

## Exemple avec état : compte bancaire

L'implémentation est dans `src/account.rs`, le scénario dans `examples/account.rs`
et le contrat indépendant dans `specs/account.cue`.

```sh
cargo run --quiet --example account > contracts/account.json
cue vet -c specs/account.cue contracts/account.json
uv run --no-project scripts/check_account.py
```

Le compte démarre à 100 unités. Chaque dépôt augmente le solde ; chaque retrait
accepté prélève le montant plus deux unités de frais. Un montant nul, un dépôt
dépassant u32::MAX ou un retrait sans fonds suffisants est refusé sans modifier
le solde. La trace couvre tous ces cas, dont un retrait de u32::MAX : le calcul
des frais lui-même ne doit pas déborder.

CUE vérifie chaque transition, son statut, la continuité entre transitions et
l'égalité du solde final avec la dernière observation. Par exemple :

| Opération | Avant | Montant | Après | Statut |
| --- | ---: | ---: | ---: | --- |
| Dépôt | 100 | 50 | 150 | ok |
| Retrait | 150 | 40 | 108 | ok |
| Retrait | 108 | 200 | 108 | insufficient_funds |
| Dépôt | 108 | 0 | 108 | zero_amount |
| Dépôt | 108 | 4294967295 | 108 | overflow |

`scripts/check_account.py`, exécuté via uv, injecte sept erreurs dans des copies
de la trace et vérifie leur rejet. Une transition valide isolément mais qui ne
part pas du solde précédent est également rejetée.

L'extraction rustdoc actuelle reste limitée à l'API `add` de premier niveau :
les types et méthodes du module bancaire ne sont pas normalisés. Cet exemple
étend la vérification du comportement, sans étendre le contrat structurel d'API.
