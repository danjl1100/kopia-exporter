## project direction
- [x] Proofread the README.md. Identify any changes up-front to help improve overall clarity.
- [x] Review the goals stated in README.md, find any that may not be helpful for the user, or any others that are missing.
- [x] Define metrics following prometheus conventions that align with the goals. Identify any commonly-used backup metrics that are not mentioned in the goals. Check back with me to discuss if there's a compelling reason to include more than stated by the goals.

## `fake-kopia`
- [ ] create a binary in `src/bin` as a stand-in for `kopia` (as it is not in the path during development)
- [ ] write a parser for the sample input file `src/sample_kopia-snapshot-list.json`
- [ ] determine useful metrics to expose to accomplish the goal stated in README.md
