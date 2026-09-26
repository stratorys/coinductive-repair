# Rust → JSON → CUE

Exemple exécutable de validation d'une API Rust et de son comportement observé.

## Exécution

Prérequis : Rust stable, Rust nightly, uv et CUE (module en version 0.17.1).
Les scripts Python sont exécutés exclusivement via `uv run`.

```sh
rustup toolchain install nightly
bash scripts/check.sh
```

Le script produit deux artefacts dans `target/contracts/` :

- `api.json` : fonctions publiques de premier niveau extraites de rustdoc ;
- `observations.json` : entrées et résultats réels de la fonction Rust `add`.

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
