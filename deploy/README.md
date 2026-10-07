# Public Render demo

The UI and Rust API share one HTTPS origin. `SELMEM_PUBLIC_DEMO=true` enables
anonymous browser sessions and ignores `SELMEM_TOKEN`. Each visitor gets a separate
memory, conversation, profile and LLM configuration. The server supplies a random
HttpOnly, SameSite=Strict cookie. Render HTTPS responses also set Secure.
There is no visitor account or shared token to enter.

## Update an existing Render deployment

1. Apply and push this patch to `main`.
2. In Render → your service → Environment, add `SELMEM_PUBLIC_DEMO=true`.
3. Delete `SELMEM_TOKEN`. Removing it from render.yaml alone may leave an existing
   environment variable in place. Public demo mode ignores it even if it remains.
4. Remove server provider keys (`SELMEM_API_KEY`) and external narrator/scorer/embedder
   configuration from this demo. Visitors' LLM configuration never inherits them.
5. Save, redeploy, then reload the UI. Test with two separate browser profiles or a
   normal window and an incognito window. Each must have an independent empty book.

For a new deployment, create a Render Blueprint from `jbsalles/Selmem`, branch
`main`, and `render.yaml`. Confirm the Free plan. Later pushes deploy after CI passes.

## Visitors

Select grok or gpt, enter your own provider API key, and click Attach. Then chat.
Without a key, the rule narrator is available; provider requests require a valid key.
Your key is sent over HTTPS to this server, kept in your session's RAM, and forwarded
only to the selected provider. It is not written to a database, exposed by GET /llm,
or inherited by another session. Changing provider clears the previous key.
The public demo restricts endpoints to the built-in OpenAI and xAI URLs.

Sessions expire after 30 minutes of inactivity and all sessions disappear when the
server restarts/redeploys/sleeps. At most 64 sessions are retained. If full, new visitors
must retry after idle sessions expire. Tabs in one browser profile share a session.
An expired session starts a new empty memory and requires entering the key again.
This is a temporary demo, unsuitable for durable experiments.

## Local check

```bash
docker build -t selmem-demo .
docker run --rm -p 10000:10000 -e SELMEM_PUBLIC_DEMO=true selmem-demo
```

Open http://localhost:10000. No shared token is required.
The default daemon mode remains a single instance with optional SELMEM_TOKEN auth;
only public demo mode enables visitor isolation.

Render Free sleeps after 15 minutes idle. Waking can take about a minute.
Provider API charges are billed to the visitor's key, independently of hosting.
References: https://render.com/docs/free and https://render.com/docs/blueprint-spec
