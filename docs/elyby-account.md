# Ely.by account sign-in

Horizon signs in to Ely.by through OAuth2 without ever receiving an Ely.by password.

## Flows

- **Device Code (default).** The launcher shows a link and a code; the player enters the code on
  Ely.by. It needs only a public client ID, no secret.
- **Authorization Code with a loopback callback (fallback).** Used only when both
  `HORIZON_ELY_CLIENT_ID` and `HORIZON_ELY_CLIENT_SECRET` are set and the code asks for it
  explicitly. The launcher listens on `http://127.0.0.1:39871/callback`
  (`HORIZON_ELY_CALLBACK_PORT` changes the port; the exact redirect URI must be registered in the
  Ely.by application). Requests whose `state` does not match are ignored.

## Application configuration

Create an application at [Ely.by application settings](https://account.ely.by/dev/applications/new)
and configure the launcher with:

```text
HORIZON_ELY_CLIENT_ID=<public client id>
HORIZON_ELY_CLIENT_SECRET=<client secret>   # only for the loopback fallback
```

The client ID may be compiled into a release build. The client secret must stay a runtime
environment variable and must never be committed or logged.

## Account and ownership

Ely.by is an alternative Minecraft-compatible authentication provider. An Ely.by account can
satisfy Horizon's play gate and start the game by itself; it is **not** proof that the player owns
a Mojang/Microsoft purchase. Microsoft and offline account behavior is unchanged.

## Session service

Ely.by sessions use `https://authserver.ely.by/session/join`; Microsoft sessions keep using the
Minecraft services discovery document. An online-mode server verifies the player against the
service it is configured for, so an Ely.by account only joins servers that accept Ely.by (for
example through authlib-injector). Ely.by accounts get no ban notice or Mojang-only features.

## Tokens

The access and refresh tokens are kept in the same keychain/file store as Microsoft credentials,
under the `elyby:` and `elyby-refresh:` prefixes. When Ely.by is unreachable during a refresh the
sign-in is kept and retried; only a refusal by Ely.by asks the player to sign in again. The game
reads a short-lived session file once, with `user_type` set to `elyby`.
