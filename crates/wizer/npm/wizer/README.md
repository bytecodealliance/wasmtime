# wizer

> Prebuilt wizer binaries available via npm

## API

```
$ npm install --save @bytecodealliance/wizer
```

```js
const execFile = require('child_process').execFile;
const wizer = require('@bytecodealliance/wizer');

execFile(wizer, ['input.wasm', '-o', 'initialized.wasm'], (err, stdout) => {
	console.log(stdout);
});
```
