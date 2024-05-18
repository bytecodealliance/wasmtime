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

const rgbToLuma = (rgb) => {
  // Use the NTSC color space (https://en.wikipedia.org/wiki/YIQ) to determine
  // the luminance of this color.  (This is an approximation using powers of two,
  // to avoid multiplications and divisions. It's good enough for our purposes.)
  let [r, g, b] = rgbToTriple(rgb);
  return (((r << 8) + (g << 9) + (b << 7)) >> 10) + (g & 31);
};
const rgbToTriple = (rgb) => [
  (rgb >> 16) & 0xff,
  (rgb >> 8) & 0xff,
  rgb & 0xff,
];
// Get the RGB color for the given offset.  (Memoize to avoid recalculating.)
const offsetToRgb = new Map();
const rgbForOffset = (offset) => {
  let color = offsetToRgb[offset];
  if (color !== undefined) return color;
  const crc24 = (crc, byte) => {
    crc ^= byte << 16;
    for (let bit = 0; bit < 8; bit++) {
      crc = ((crc << 1) ^ (crc & 0x800000 ? 0xfa5711 : 0)) & 0xffffff;
    }
    return crc;
  };
  let orig_offset = offset;
  for (color = offset; offset; offset >>= 8)
    color = crc24(color, offset & 0xff);
  color = rgbToLuma(color) > 127 ? color ^ 0xa5a5a5 : color;
  offsetToRgb[orig_offset] = color;
  return color;
};
const rgbToCss = (rgb) => `rgba(${rgbToTriple(rgb).join(",")})`;
const adjustColorForOffset = (element, offset) => {
  let backgroundColor = rgbForOffset(offset);
  element.style.backgroundColor = rgbToCss(backgroundColor);
  element.classList.add(
    rgbToLuma(backgroundColor) > 128 ? "dark-text" : "light-text",
  );
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

const linkElements = (element) => {
  const selector = (offset) =>
    document.querySelectorAll(`[data-wasm-offset="${offset}"]`);
  const eachElementWithSameWasmOff = (event, closure) => {
    let offset = event.target.dataset.wasmOffset;
    if (offset !== null) {
      for (const elem of selector(offset)) closure(elem);
    }
  };
  element.addEventListener(
    "click",
    (event) => {
      document.getElementById("bridge").style.display = "none";
      eachElementWithSameWasmOff(event, (elem) => {
        elem.scrollIntoView({
          behavior: "smooth",
          block: "center",
          inline: "nearest",
        });
      });
    },
    { passive: true },
  );
  element.addEventListener("mouseenter", (event) => {
    let offset = event.target.dataset.wasmOffset;
    if (offset === null) return;
    // Gather all elements related to the desired offset.  Put the one in the WAT
    // view first, and then all the others subsequently; this is done so we can
    // calculate the polygon to bridge the WAT and the ASM views.
    //
    // FIXME: optimize for the common case where selector() returns only two
    // elements!
    let elems = Array.from(selector(offset).entries()).map((elem) => {
      let [_discard, node] = elem;
      return node;
    });
    elems.sort((elem0, elem1) => {
      let rect0 = elem0.getBoundingClientRect();
      let rect1 = elem1.getBoundingClientRect();
      return rect0.x - rect1.x;
    });
    let rect0 = elems[0].getBoundingClientRect();
    let rect1 = elems[1].getBoundingClientRect();
    let points = elems
      .slice(1)
      .map((elem) => {
        let rect = elem.getBoundingClientRect();
        return `0 ${rect0.y - 8}px, 100% ${rect.y - 8}px, 100% ${rect.bottom + 8}px, 0 ${rect0.bottom + 8}px`;
      })
      .join(",");
    let bridge = document.getElementById("bridge");
    bridge.style.display = "block";
    bridge.style.left = `${rect0.width}px`;
    bridge.style.width = `${rect1.left - rect0.width}px`;
    bridge.style.clipPath = `polygon(${points})`;
    bridge.style.backgroundColor = elems[0].style.backgroundColor;
    for (const elem of elems) {
      // TODO: if any of these elems is out of view, show in the pop-up there it is (up or down)
      elem.setAttribute("title", `WASM offset @ ${offset}`);
      elem.classList.add("hovered");
      elem.style.outline = `8px solid ${rgbToCss(rgbForOffset(offset))}`;
    }
  });
  element.addEventListener("mouseleave", (event) => {
    document.getElementById("bridge").style.display = "none";
    eachElementWithSameWasmOff(event, (elem) => {
      elem.removeAttribute("title");
      elem.classList.remove("hovered");
      elem.style.outline = "";
    });
  });
};

const createDivForCode = () => {
  let div = document.createElement("div");
  div.classList.add("highlight");
  return div;
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

  let currentBlock = createDivForCode();
  let disasmBuffer = [];
  let lastOffset = null;

  const addCurrentBlock = (offset) => {
    currentBlock.setAttribute("data-wasm-offset", offset);

    if (offset !== null) {
      adjustColorForOffset(currentBlock, offset);
      linkElements(currentBlock);
    }

    currentBlock.innerText = disasmBuffer.join("\n");
    funcElem.appendChild(currentBlock);
    disasmBuffer = [];
  };

  for (const inst of func.instructions) {
    disasmBuffer.push(
      `${renderAddress(inst.address)}    ${renderBytes(inst.bytes)}    ${renderInst(inst.mnemonic, inst.operands)}`,
    );
    if (lastOffset !== inst.wasm_offset) {
      addCurrentBlock(lastOffset);
      currentBlock = createDivForCode();
      lastOffset = inst.wasm_offset;
    }
  }
  addCurrentBlock(lastOffset);

  document.getElementById("asm").appendChild(funcElem);
}

// Render the WAT.
for (const chunk of state.wat.chunks) {
  if (chunk.wasm_offset === null) continue;
  const block = createDivForCode();
  block.dataset.wasmOffset = chunk.wasm_offset;
  block.innerText = chunk.wat;

  if (offsetToRgb[chunk.wasm_offset] !== undefined) {
    adjustColorForOffset(block, chunk.wasm_offset);
    linkElements(block);
  }

  document.getElementById("wat").appendChild(block);
}
