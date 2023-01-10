const core = require('@actions/core');
const tc = require('@actions/tool-cache');

function z3URL(architecture, version, distribution) {
    let path = "https://github.com/Z3Prover/z3/releases/download/z3-" + version;
    let file = "z3-" + version + "-" + architecture + "-" + distribution + ".zip";
    return { path: path, file: file };
}

(async function() {
    try {
        const architecture = core.getInput('architecture', { required: true });
        const distribution = core.getInput('distribution', { required: true });
        const version = core.getInput('version', { required: true });

        const url = z3URL(architecture, version, distribution);
        const path = await tc.downloadTool(url.path + "/" + url.file);
        const dir = await tc.extractZip(path)
        const cachedPath = await tc.cacheDir(dir, 'z3', version);
        core.addPath(cachedPath + "/" + url.file.replace(/\.zip$/, "") + "/bin");
	core.exportVariable("CPATH", cachedPath + "/" + url.file.replace(/\.zip$/, "") + "/include");
    } catch (error) {
        core.setFailed(error.message);
    }
})();
