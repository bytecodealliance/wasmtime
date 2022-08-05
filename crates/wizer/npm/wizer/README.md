# wizer

> Prebuilt wizer binaries available via npm

## API

```
$ npm install --save @bytecode-alliance/wizer
```

```js
const execFile = require('child_process').execFile;
const wizer = require('@bytecode-alliance/wizer');

execFile(wizer, ['input.wasm', '-o', 'initialized.wasm'], (err, stdout) => {
	console.log(stdout);
});
```
