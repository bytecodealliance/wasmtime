/* eslint-disable no-restricted-syntax */
/* eslint-disable import/no-extraneous-dependencies */
/* eslint-disable no-use-before-define */
const path = require('path');
const fs = require('fs');
const chalk = require('chalk');
const zkasm = require('@0xpolygonhermez/zkasmcom');
const smMain = require('@0xpolygonhermez/zkevm-proverjs/src/sm/sm_main/sm_main');
const {
    compile,
    newCommitPolsArray
} = require('pilcom');
const buildPoseidon = require('@0xpolygonhermez/zkevm-commonjs').getPoseidon;

const emptyInput = require('@0xpolygonhermez/zkevm-proverjs/test/inputs/empty_input.json');

const {
    argv
} = require('yargs')
    .alias('v', 'verbose');

// Global paths to build Main PIL to fill polynomials in tests
const pathMainPil = path.join(__dirname, 'node_modules/@0xpolygonhermez/zkevm-proverjs/pil/main.pil');
const fileCachePil = path.join(__dirname, 'node_modules/@0xpolygonhermez/zkevm-proverjs/cache-main-pil.json');

async function main() {
    // Compile pil
    console.log(chalk.yellow('--> Compile PIL'));
    const cmPols = await compilePil();

    // Get all zkasm files
    const pathZkasm = path.join(process.cwd(), process.argv[2]);
    const files = await getTestFiles(pathZkasm);

    let hasUnexpectedFailures = false;
    // Run all zkasm files
    // eslint-disable-next-line no-restricted-syntax
    console.log(chalk.yellow('--> Start running zkasm files'));
    for (const file of files) {
        if (file.includes('ignore'))
            continue;

        let shouldFail = file.split("/").pop().startsWith("_should_fail_");
        let testFailed = await runTest(file, cmPols);
        hasUnexpectedFailures |= (testFailed && !shouldFail) || (shouldFail && !testFailed);
    }
    if (hasUnexpectedFailures) {
        process.exit(1);
    }
}

async function compilePil() {
    if (!fs.existsSync(fileCachePil)) {
        const poseidon = await buildPoseidon();
        const {
            F
        } = poseidon;
        const pilConfig = {
            defines: {
                N: 4096
            },
            namespaces: ['Main', 'Global'],
            disableUnusedError: true,
        };
        const p = await compile(F, pathMainPil, null, pilConfig);
        fs.writeFileSync(fileCachePil, `${JSON.stringify(p, null, 1)}\n`, 'utf8');
    }

    const pil = JSON.parse(fs.readFileSync(fileCachePil));

    return newCommitPolsArray(pil);
}

// Get all zkasm test files
function getTestFiles(pathZkasm) {
    // check if path provided is a file or a directory
    const stats = fs.statSync(pathZkasm);

    if (!stats.isDirectory()) {
        return [pathZkasm];
    }

    const filesNames = fs.readdirSync(pathZkasm).filter((name) => name.endsWith('.zkasm'));

    return filesNames.map((fileName) => path.join(pathZkasm, fileName));
}

// returns true if test succeed and false if test failed
async function runTest(pathTest, cmPols) {
    // Compile rom
    const configZkasm = {
        defines: [],
        allowUndefinedLabels: true,
        allowOverwriteLabels: true,
    };

    const config = {
        debug: true,
        stepsN: 8388608,
        assertOutputs: false,
    };
    let failed = false;
    // execute zkasm tests
    try {
        const rom = await zkasm.compile(pathTest, null, configZkasm);
        const result = await smMain.execute(cmPols.Main, emptyInput, rom, config);
        console.log(chalk.green('   --> pass'), pathTest);
        if (argv.verbose) {
            console.log(chalk.blue('   --> verbose'));
            console.log(chalk.blue('        --> counters'));
            console.log(result.counters);
            console.log(chalk.blue('        --> outputs'));
            console.log(result.output);
            console.log(chalk.blue('        --> logs'));
            console.log(result.logs);
        }
    } catch (e) {
        console.log(chalk.red('   --> fail'), pathTest);
        console.log(e);
        failed = true;
    }
    return failed;
}

main();
