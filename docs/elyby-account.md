# Ely.by account sign-in

Horizon can sign in to Ely.by through OAuth2 without receiving an Ely.by password. The launcher
opens the Ely.by authorization page and listens for the authorization callback on
`http://127.0.0.1:39871/callback`. The callback port can be changed with
`HORIZON_ELY_CALLBACK_PORT`, but the selected URI must also be registered in the Ely.by
application.

## Application configuration

Create a Website application at [Ely.by application settings](https://account.ely.by/dev/applications/new)
with the exact redirect URI:

```text
http://127.0.0.1:39871/callback
```

Configure the launcher with:

```text
HORIZON_ELY_CLIENT_ID=<public client id>
HORIZON_ELY_CLIENT_SECRET=<client secret>
```

The client ID may be compiled into a release build, but the client secret must remain a runtime
environment variable and must never be committed to the repository or printed in logs.

## Account and ownership

Ely.by authentication provides an alternative Minecraft-compatible account/session, so an Ely.by
account can satisfy Horizon's play gate and start the game by itself. This is intentionally
different from Microsoft ownership: Ely.by is treated as an alternate authentication provider,
not as proof that the Microsoft account owns a Mojang/Microsoft purchase.

Microsoft and offline account behavior remains unchanged.

## Session service

Ely.by sessions use `https://authserver.ely.by/session/join`. Microsoft sessions continue to use
the Minecraft services discovery document. Horizon's game reads the short-lived session file once;
access tokens are kept in the same keychain/file-store mechanism as Microsoft credentials.
