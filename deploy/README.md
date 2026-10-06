# Free Render test environment

The Rust daemon serves both the UI and its API on the same HTTPS origin.
This Blueprint explicitly selects Render's Free plan in Frankfurt. There is no paid
persistent disk, database, or external LLM configured by default.

## First deployment

1. Apply the deployment patch to your updated checkout and commit the files.
2. Create a new test branch and push it:

   ```bash
   git switch -c staging
   git push -u origin staging
   ```

   If `staging` already exists, switch to it and merge or cherry-pick the deployment
   commit rather than recreating or force-pushing it.
3. Sign in to https://dashboard.render.com and connect your GitHub account.
4. Select **New → Blueprint**, choose `jbsalles/Selmem`, and select branch `staging`.
5. Render reads `render.yaml`. Confirm that the proposed service uses **Free**,
   then create it. Wait for the deployment to complete.
6. Open the service's actual `https://…onrender.com` URL. Both `/` (UI) and `/health`
   should respond. In Render's **Environment** settings, copy the generated
   `SELMEM_TOKEN` and paste it into the UI's token field. Share it only with testers.

Later commits pushed to `staging` redeploy automatically after GitHub checks pass.
The Rust and container workflows run on this branch and on pull requests.
The first Blueprint creation starts an initial deployment; check its CI results too.

## What this demo does

Without LLM configuration, SelMem uses RuleNarrator. Memory mechanisms work, but
this is not a demo of a language model's conversational capabilities.
All testers share one memory and one profile. Reset/profile changes affect everyone.

The database lives in `/var/lib/selmem/demo.db`, on the service's ephemeral filesystem.
Render's Free service sleeps after 15 minutes without traffic; waking it can take
about a minute. Sleep, replacement, restart, or redeployment can erase the memory.
This reset is an infrastructure limitation, separate from SelMem's selective forgetting.
Do not use this instance for long-term experiments or durable personal memories.

## Optional external LLM

In Render's Environment settings, add:

- `SELMEM_LLM`: provider chat-completions URL
- `SELMEM_MODEL`: model identifier
- `SELMEM_API_KEY`: provider key, stored as a secret

Save and redeploy. The hosting plan remains Free; provider API usage can cost money.
No provider key belongs in GitHub, the Docker image, or `render.yaml`.

## Local container check

```bash
docker build -t selmem-demo .
docker run --rm -p 10000:10000 -e SELMEM_TOKEN=local-demo-token selmem-demo
```

Open http://localhost:10000 and enter `local-demo-token` in the UI.
A volume can preserve local data, but the free Render Blueprint deliberately has none.

References: https://render.com/docs/free and https://render.com/docs/blueprint-spec
