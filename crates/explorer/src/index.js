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
    // CRC computation adapted from Wikipedia[1] (shift-register based division versions.)
    // [1] https://en.m.wikipedia.org/wiki/Computation_of_cyclic_redundancy_checks
    crc ^= byte << 16;
    for (let bit = 0; bit < 8; bit++) {
      crc = ((crc << 1) ^ (crc & 0x800000 ? 0xfa5711 : 0)) & 0xffffff;
    }
    return crc;
  };
  let orig_offset = offset;
  for (color = offset; offset; offset >>= 8)
    color = crc24(color, offset & 0xff);
  // Avoid colors that are too close to white.
  color = rgbToLuma(color) > 200 ? color ^ 0xa5a5a5 : color;
  offsetToRgb[orig_offset] = color;
  return color;
};
const rgbToCss = (rgb) => `rgba(${rgbToTriple(rgb).join(",")})`;
const rgbDarken = (rgb) => {
  let [r, g, b] = rgbToTriple(rgb);
  return (
    ((r - Math.min(r, 0x20)) << 16) |
    ((g - Math.min(g, 0x20)) << 8) |
    (b - Math.min(b, 0x20))
  );
};
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
    let elems = selector(offset);
    let wat_elem, asm_elems;
    if (elems.length == 2) {
      let rect0 = elems[0].getBoundingClientRect();
      let rect1 = elems[1].getBoundingClientRect();
      if (rect0.x < rect1.x) {
        wat_elem = elems[0];
        asm_elems = [elems[1]];
      } else {
        wat_elem = elems[1];
        asm_elems = [elems[0]];
      }
    } else if (elems.length < 2) {
      return;
    } else {
      elems = Array.from(selector(offset).entries()).map((elem) => elem[1]);
      elems.sort(
        (elem0, elem1) =>
          elem0.getBoundingClientRect().x - elem1.getBoundingClientRect().x,
      );
      wat_elem = elems[0];
      asm_elems = elems.slice(1);
    }
    let bridgeWidth = 16;
    let wat_rect = wat_elem.getBoundingClientRect();
    let points = asm_elems
      .map((elem) => {
        let rect = elem.getBoundingClientRect();
        bridgeWidth = rect.left - wat_rect.width;
        return `0 ${wat_rect.y - 2}px, 100% ${rect.y - 2}px, 100% ${rect.bottom + 2}px, 0 ${wat_rect.bottom + 2}px`;
      })
      .join(",");
    let bridge = document.getElementById("bridge");
    bridge.style.display = "block";
    bridge.style.left = `${wat_rect.width}px`;
    bridge.style.width = `${bridgeWidth}px`;
    bridge.style.clipPath = `polygon(${points})`;
    bridge.style.backgroundColor = wat_elem.style.backgroundColor;
    let outline = `2px solid ${rgbToCss(rgbDarken(rgbForOffset(offset)))}`;
    for (const elem of elems) {
      // TODO: if any of these elems is out of view, show in the pop-up there it is (up or down)
      elem.setAttribute("title", `WASM offset @ ${offset}`);
      elem.classList.add("hovered");
      elem.style.outline = outline;
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
    if (lastOffset !== inst.wasm_offset) {
      addCurrentBlock(lastOffset);
      currentBlock = createDivForCode();
      lastOffset = inst.wasm_offset;
    }
    disasmBuffer.push(
      `${renderAddress(inst.address)}    ${renderBytes(inst.bytes)}    ${renderInst(inst.mnemonic, inst.operands)}`,
    );
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
