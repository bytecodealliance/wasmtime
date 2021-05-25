const core = require('@actions/core');
const path = require("path");
const fs = require("fs");
const github = require('@actions/github');
const glob = require('glob');

function sleep(milliseconds) {
  return new Promise(resolve => setTimeout(resolve, milliseconds))
}

async function runOnce() {
  // Load all our inputs and env vars. Note that `getInput` reads from `INPUT_*`
  const files = core.getInput('files');
  const token = core.getInput('token');
  const slug = process.env.GITHUB_REPOSITORY;
  const owner = slug.split('/')[0];
  const repo = slug.split('/')[1];
  const sha = process.env.GITHUB_SHA;
  let name = 'dev';
  if (process.env.GITHUB_REF.startsWith('refs/tags/v')) {
    name = process.env.GITHUB_REF.substring(10);
  }

  core.info(`files: ${files}`);
  core.info(`name: ${name}`);
  core.info(`token: ${token}`);

  const octokit = new github.GitHub(token);

  // For the `dev` release we may need to update the tag to point to the new
  // commit on this branch. All other names should already have tags associated
  // with them.
  if (name == 'dev') {
    let tag = null;
    try {
      tag = await octokit.request("GET /repos/:owner/:repo/git/refs/tags/:name", { owner, repo, name });
      core.info(`found existing tag`);
      console.log("tag: ", JSON.stringify(tag.data, null, 2));
    } catch (e) {
      // ignore if this tag doesn't exist
      core.info(`no existing tag found`);
    }

    if (tag === null || tag.data.object.sha !== sha) {
      core.info(`updating existing tag or creating new one`);
      // Delete the previous release for this tag, if any
      try {
        core.info(`fetching release for ${name}`);
        const release = await octokit.repos.getReleaseByTag({ owner, repo, tag: name });
        core.info(`deleting release ${release.data.id}`);
        await octokit.repos.deleteRelease({ owner, repo, release_id: release.data.id });
      } catch (e) {
        // ignore, there may not have been a release
        console.log("ERROR: ", JSON.stringify(e, null, 2));
      }

      try {
        core.info(`updating dev tag`);
        await octokit.git.updateRef({
            owner,
            repo,
            ref: 'tags/dev',
            sha,
            force: true,
        });
      } catch (e) {
        console.log("ERROR: ", JSON.stringify(e, null, 2));
        core.info(`creating dev tag`);
        try {
          await octokit.git.createTag({
            owner,
            repo,
            tag: 'dev',
            message: 'dev release',
            object: sha,
            type: 'commit',
          });
        } catch (e) {
          // we might race with others, so assume someone else has created the
          // tag by this point.
        }
      }
    } else {
      core.info(`existing tag works`);
    }
  }

  // Try to load the release for this tag, and if it doesn't exist then make a
  // new one. We might race with other builders on creation, though, so if the
  // creation fails try again to get the release by the tag.
  let release = null;
  try {
    core.info(`fetching release`);
    release = await octokit.repos.getReleaseByTag({ owner, repo, tag: name });
  } catch (e) {
    console.log("ERROR: ", JSON.stringify(e, null, 2));
    core.info(`creating a release`);
    try {
      release = await octokit.repos.createRelease({
        owner,
        repo,
        tag_name: name,
        prerelease: name === 'dev',
      });
    } catch(e) {
      console.log("ERROR: ", JSON.stringify(e, null, 2));
      core.info(`fetching one more time`);
      release = await octokit.repos.getReleaseByTag({ owner, repo, tag: name });
    }
  }
  console.log("found release: ", JSON.stringify(release.data, null, 2));

  // Upload all the relevant assets for this release as just general blobs.
  for (const file of glob.sync(files)) {
    const size = fs.statSync(file).size;
    core.info(`upload ${file}`);
    await octokit.repos.uploadReleaseAsset({
      data: fs.createReadStream(file),
      headers: { 'content-length': size, 'content-type': 'application/octet-stream' },
      name: path.basename(file),
      url: release.data.upload_url,
    });
  }
}

async function run() {
  const retries = 10;
  for (let i = 0; i < retries; i++) {
    try {
      await runOnce();
      break;
    } catch (e) {
      if (i === retries - 1)
        throw e;
      logError(e);
      console.log("RETRYING after 10s");
      await sleep(10000)
    }
  }
}

function logError(e) {
  console.log("ERROR: ", e.message);
  try {
    console.log(JSON.stringify(e, null, 2));
  } catch (e) {
    // ignore json errors for now
  }
  console.log(e.stack);
}

run().catch(err => {
  logError(err);
  core.setFailed(err.message);
});
