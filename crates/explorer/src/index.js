/* global window, document */

/*** State *********************************************************************/

class State {
  constructor(wat, asm) {
    this.wat = wat;
    this.asm = asm;
  }
}

const state = (window.STATE = new State(window.WAT, window.ASM));

/*** Colors for Offsets **********************************************************/

const offsetToRgb = new Map();

// Get the RGB color for the given offset.  (Memoize to avoid recalculating.)
const rgbForOffset = (offset) => {
  if (offsetToRgb.has(offset)) {
    return offsetToRgb.get(offset);
  }
  const crc24 = (crc, byte) => {
    crc ^= byte << 16;
    for (let bit = 0; bit < 8; bit++) {
      crc = (crc & 0x800000 ? (crc << 1) ^ 0xfa5711 : crc << 1) & 0xffffff;
    }
    return crc;
  };
  let color;
  for (color = offset; offset; offset >>= 8)
    color = crc24(color, offset & 0xff);
  color = `${(color >> 16) & 0xff}, ${(color >> 8) & 0xff}, ${color & 0xff}`;
  offsetToRgb.set(offset, color);
  return color;
};

const dimColorForOffset = (offset) => `rgba(${rgbForOffset(offset)}, 0.3)`;
const brightColorForOffset = (offset) => `rgba(${rgbForOffset(offset)}, 0.7)`;

// Get WAT chunk elements by Wasm offset.
const watByOffset = new Map();

// Get asm instruction elements by Wasm offset.
const asmByOffset = new Map();

// Get all (WAT chunk or asm instruction) elements by offset.
const anyByOffset = new Map();

const addWatElem = (offset, elem) => {
  if (!watByOffset.has(offset)) {
    watByOffset.set(offset, []);
  }
  watByOffset.get(offset).push(elem);

  if (!anyByOffset.has(offset)) {
    anyByOffset.set(offset, []);
  }
  anyByOffset.get(offset).push(elem);
};

const addAsmElem = (offset, elem) => {
  if (!asmByOffset.has(offset)) {
    asmByOffset.set(offset, []);
  }
  asmByOffset.get(offset).push(elem);

  if (!anyByOffset.has(offset)) {
    anyByOffset.set(offset, []);
  }
  anyByOffset.get(offset).push(elem);
};

/*** Event Handlers ************************************************************/

const watElem = document.getElementById("wat");
watElem.addEventListener(
  "click",
  (event) => {
    if (event.target.dataset.wasmOffset == null) {
      return;
    }

    const offset = parseInt(event.target.dataset.wasmOffset);
    if (!asmByOffset.get(offset)) {
      return;
    }

    const firstAsmElem = asmByOffset.get(offset)[0];
    firstAsmElem.scrollIntoView({
      behavior: "smooth",
      block: "center",
      inline: "nearest",
    });
  },
  { passive: true },
);

const asmElem = document.getElementById("asm");
asmElem.addEventListener(
  "click",
  (event) => {
    if (event.target.dataset.wasmOffset == null) {
      return;
    }

    const offset = parseInt(event.target.dataset.wasmOffset);
    if (!watByOffset.get(offset)) {
      return;
    }

    const firstWatElem = watByOffset.get(offset)[0];
    firstWatElem.scrollIntoView({
      behavior: "smooth",
      block: "center",
      inline: "nearest",
    });
  },
  { passive: true },
);

const onMouseEnter = (event) => {
  if (event.target.dataset.wasmOffset == null) {
    return;
  }

  const offset = parseInt(event.target.dataset.wasmOffset);
  const color = brightColorForOffset(offset);
  for (const elem of anyByOffset.get(offset)) {
    elem.style.backgroundColor = color;
  }
};

const onMouseLeave = (event) => {
  if (event.target.dataset.wasmOffset == null) {
    return;
  }

  const offset = parseInt(event.target.dataset.wasmOffset);
  const color = dimColorForOffset(offset);
  for (const elem of anyByOffset.get(offset)) {
    elem.style.backgroundColor = color;
  }
};

/*** Rendering *****************************************************************/

const repeat = (s, n) => {
  return s.repeat(n >= 0 ? n : 0);
};

const renderAddress = (addr) => {
  let hex = addr.toString(16);
  return repeat("0", 8 - hex.length) + hex;
};

const renderBytes = (bytes) => {
  let s = "";
  for (let i = 0; i < bytes.length; i++) {
    if (i != 0) {
      s += " ";
    }
    const hexByte = bytes[i].toString(16);
    s += hexByte.length == 2 ? hexByte : "0" + hexByte;
  }
  return s + repeat(" ", 30 - s.length);
};

const renderInst = (mnemonic, operands) => {
  if (operands.length == 0) {
    return mnemonic;
  } else {
    return mnemonic + " " + operands;
  }
};

// Render the ASM.

for (const func of state.asm.functions) {
  const funcElem = document.createElement("div");

  const funcHeader = document.createElement("h3");
  let func_name =
    func.name === null ? `function[${func.func_index}]` : func.name;
  let demangled_name =
    func.demangled_name !== null ? func.demangled_name : func_name;
  funcHeader.textContent = `Disassembly of function <${demangled_name}>:`;
  funcHeader.title = `Function ${func.func_index}: ${func_name}`;
  funcElem.appendChild(funcHeader);

  const bodyElem = document.createElement("pre");
  for (const inst of func.instructions) {
    const instElem = document.createElement("span");
    instElem.textContent = `${renderAddress(inst.address)}    ${renderBytes(inst.bytes)}    ${renderInst(inst.mnemonic, inst.operands)}\n`;
    if (inst.wasm_offset != null) {
      instElem.setAttribute("data-wasm-offset", inst.wasm_offset);
      instElem.style.backgroundColor = dimColorForOffset(inst.wasm_offset);
      instElem.addEventListener("mouseenter", onMouseEnter);
      instElem.addEventListener("mouseleave", onMouseLeave);
      addAsmElem(inst.wasm_offset, instElem);
    }
    bodyElem.appendChild(instElem);
  }
  funcElem.appendChild(bodyElem);

  asmElem.appendChild(funcElem);
}

// Render the WAT.

for (const chunk of state.wat.chunks) {
  const chunkElem = document.createElement("span");
  if (chunk.wasm_offset != null) {
    chunkElem.dataset.wasmOffset = chunk.wasm_offset;
    chunkElem.style.backgroundColor = dimColorForOffset(chunk.wasm_offset);
    chunkElem.addEventListener("mouseenter", onMouseEnter);
    chunkElem.addEventListener("mouseleave", onMouseLeave);
    addWatElem(chunk.wasm_offset, chunkElem);
  }
  chunkElem.textContent = chunk.wat;
  watElem.appendChild(chunkElem);
}
