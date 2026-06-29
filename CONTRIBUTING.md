# Contributing Guidelines

## Submitting a pull request

If the pull request is not yet ready to be reviewed by the maintainers, ensure it is marked as "Draft". When it is ready, mark it as "Ready for Review".

Before marking a pull request as ready for review, ensure:

* Commits are cleanly separated and have useful (but concise) messages that explain WHAT changed and WHY.
* A changelog entry has been added to CHANGELOG.md under `## Unreleased`. Changelogs should be concise and focused on the end user.
* Code has been appropriately documented (doc comments, etc.) Documentation should be clear and thorough but not excessively verbose or repetitive.
* Code is appropriately formatted with `cargo fmt`.
* Test coverage is excellent and passes with `--all-features` enabled.
* Reference related issues with "closes #N" at the bottom of commit messages.
* If an issue or pull request has been created with assistance from AI tooling, the contributor MUST review their contributions before posting them.

## AI-assisted contributions policy

The following policy is adapted from [Astral's AI contributions policy](https://github.com/astral-sh/.github/blob/main/AI_POLICY.md)

We allow using AI (e.g. LLMs) as tools for coding. However, you remain responsible for any code you publish, and we are responsible for any code we merge and release.

Contributing to this project means vouching for the quality, license compliance, and utility of your submission.

If you are opening an issue or pull request, we expect you to be able to explain the issue and/or proposed changes in your own words. This includes the issue / pull request body and responses to questions. **Do not copy responses from the AI when replying to questions from maintainers.**

Autonomous AI agents should not be used to reply to questions from maintainers. If you wish to include context from an interaction with AI in your comments, it must be in a quote block (e.g., using >) and disclosed as such. It must also be reviewed by the contributor for relevance and accuracy and accompanied by human commentary explaining the implications of the context.

Contributions which appear in violation of this policy may be be closed, perhaps without notice.
