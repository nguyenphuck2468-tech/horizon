# Localization quality audit

## Scope

This is a staged quality pass over all 86 shipped locale tables in `launcher/lang/`.
English remains the source language in the Rust code; there is no `en.json`.

## Phase 1: technical terminology sweep

The first pass targets recurring UI terms and technical strings that were mistranslated literally or interpreted in the wrong context:

- Preserve product and technology names such as Tailscale, Mesa, Vulkan, Mojang, and Minecraft.
- Preserve commands and configuration tokens exactly, including `nice -n 5` and `DRI_PRIME=1`.
- Translate JAR references as archive files, not kitchen jars.
- Translate GPU/card references as graphics hardware, not playing cards.
- Keep Minecraft launcher concepts distinct: an instance is a game installation/profile, a skin is a player skin, and a client is software rather than a customer.
- Use the correct UI action for minimizing a window, rather than a generic reduction.
- Describe the Mojang-shipped Minecraft version as the original/released version, not as a physical shipment.

The pass preserves every English key and every format placeholder. It changes values only.

## Validation rules

For every locale:

1. The JSON must parse.
2. English keys must remain unchanged.
3. Every `{placeholder}` must be preserved exactly.
4. Technical commands, environment variables, file extensions, and product names must not be corrupted.
5. Wording must be judged in the actual launcher context, not translated word-for-word.

## Remaining work

This is not a claim that every sentence in every locale has received a native-speaker review. The first pass fixes a targeted set of high-impact terminology errors across all 86 files. The remaining strings require a broader contextual review, followed by native-speaker review where idiom or low-resource language coverage makes machine-assisted judgement unreliable.

Do not treat an untranslated technical term as an error when it is the established term used by that language's Minecraft community.
