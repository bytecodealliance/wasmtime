/*** LRU Cache *****************************************************************/

class Cache {
  constructor(size, getFunc) {
    // Maps preserve the insertion order, so we can use it to implement a naïve LRU
    // cache.
    this.cache = new Map();
    this.cacheSize = size;
    this.getFunc = getFunc;
  }

  get(key) {
    let v = this.cache.get(key);
    if (v !== undefined) {
      // Remove the found element from the cache so it can be inserted it again
      // at the end before returning.
      this.cache.delete(key);
    } else {
      v = this.getFunc(key);
      if (this.cache.size > this.cache.cacheSize) {
        // Evict the oldest item from the cache.
        this.cache.delete(this.cache.keys().next().value);
      }
    }
    this.cache.set(key, v);
    return v;
  }
}

/*** Colors for Offsets **********************************************************/

const rgbToLuma = rgb => {
  // Use the NTSC color space (https://en.wikipedia.org/wiki/YIQ) to determine
  // the luminance (Y) of this color.  (This is an approximation using powers of two,
  // to avoid multiplications and divisions. It's not accurate, but it's good enough
  // for our purposes.)
  let [r, g, b] = rgbToTriple(rgb);
  return (((r << 8) + (g << 9) + (b << 7)) >> 10) + (g & 31);
};

// Convert a color as a 24-bit number into a list with 3 elements: R, G, and B,
// each ranging [0, 255].
const rgbToTriple = rgb => [(rgb >> 16) & 0xff, (rgb >> 8) & 0xff, rgb & 0xff];

// Use CRC24 as a way to calculate a color for a given Wasm offset. This
// particular algorithm has been chosen because it produces bright, vibrant
// colors, that don't repeat often, and is easily implementable.
const calculateRgbForOffset = offset => {
  const crc24 = (crc, byte) => {
    // CRC computation adapted from Wikipedia[1] (shift-register based division versions.)
    // [1] https://en.m.wikipedia.org/wiki/Computation_of_cyclic_redundancy_checks
    crc ^= byte << 16;
    for (let bit = 0; bit < 8; bit++) {
      crc = ((crc << 1) ^ (crc & 0x800000 ? 0xfa5711 : 0)) & 0xffffff;
    }
    return crc;
  };

  // Feed the offset into the CRC24 algorithm, one byte at a time.
  let color = offset;
  while (offset) {
    color = crc24(color, offset & 0xff);
    offset >>= 8;
  }

  // Avoid colors that are too close to white.  Flip some bits around
  // so that the color components are more pronounced.
  return rgbToLuma(color) > 200 ? color ^ 0xa5a5a5 : color;
};

// Memoize all colors for a given Wasm offset. Cache isn't used here since,
// when rendering the Wat side, we use the fact that if a color has not been
// assigned during the rendering of the Native Asm side, that block of Wasm
// instructions isn't colored.
let offsetToRgb = new Map();
const rgbForOffset = offset => {
  let rgb = offsetToRgb.get(offset);
  if (rgb === undefined) {
    rgb = calculateRgbForOffset(offset);
    offsetToRgb.set(offset, rgb);
  }
  return rgb;
};

// Convert a color in a 24-bit number to a string suitable for CSS styling.
const rgbToCss = rgb => `rgba(${rgbToTriple(rgb).join(",")})`;

// Darkens a color in a 24-bit number slightly by subtracting at most 0x20
// from each color component; e.g. RGB(175, 161, 10) becomes RGB(143, 129, 0).
// This loses some color information, but it's good enough for our use case here.
const rgbDarken = rgb => {
  let [r, g, b] = rgbToTriple(rgb);
  return (
    ((r - Math.min(r, 0x20)) << 16) |
    ((g - Math.min(g, 0x20)) << 8) |
    (b - Math.min(b, 0x20))
  );
};

// Adjust the color styles of a DOM element for a given Wasm offset.
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

const renderAddress = addr => {
  let hex = addr.toString(16);
  return repeat("0", 8 - hex.length) + hex;
};

const renderBytes = bytes => {
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

// Connects callbacks to mouse hovering events so elements are properly highlighted when
// hovered, and the bridging element is drawn between the instruction lists.
const linkedElementCache = new Cache(256, offset =>
  document.querySelectorAll(`[data-wasm-offset="${offset}"]`),
);
const linkElements = element => {
  const eachElementWithSameWasmOff = (event, closure) => {
    let offset = event.target.dataset.wasmOffset;
    if (offset !== null) {
      // Run the loop inside an animation frame.  Since we're modifying the DOM,
      // do so when the browser has some breathing room.
      window.requestAnimationFrame(() => {
        linkedElementCache.get(offset).forEach(closure);
      });
    }
  };

  element.addEventListener(
    "click",
    event => {
      document.getElementById("bridge").style.display = "none";
      eachElementWithSameWasmOff(event, elem => {
        if (elem === event.target) return; // Only scroll into view the other elements.
        elem.scrollIntoView({
          behavior: "smooth",
          block: "center",
          inline: "nearest",
        });
      });
    },
    { passive: true },
  );

  element.addEventListener("mouseenter", event => {
    let offset = event.target.dataset.wasmOffset;
    if (offset === null) return;

    // Gather all elements related to the desired offset.  Put the one in the WAT
    // view first, and then all the others subsequently; this is done so we can
    // calculate the polygon to bridge the WAT and the ASM views.
    let elems = linkedElementCache.get(offset);
    if (elems.length < 2) return;

    let watElem, asmElems;
    if (elems.length == 2) {
      // The most common case: only two elements matching a given Wasm offset, so
      // no need to convert the NodeListOf returned by selector() to an array like
      // in the general case below so we can sort by the X position.
      let rect0 = elems[0].getBoundingClientRect();
      let rect1 = elems[1].getBoundingClientRect();
      if (rect0.x < rect1.x) {
        watElem = elems[0];
        asmElems = [elems[1]];
      } else {
        watElem = elems[1];
        asmElems = [elems[0]];
      }
    } else {
      elems = Array.from(elems).sort(
        (elem0, elem1) =>
          elem0.getBoundingClientRect().x - elem1.getBoundingClientRect().x,
      );
      watElem = elems[0];
      asmElems = elems.slice(1);
    }

    // Calculate all the points that form the polygon that's drawn between
    // the Wasm code and the Native Asm code.  Start with a width of 16px,
    // but recalculate it based on the position of the list elements as we
    // iterate over them. One 4-point polygon will be constructed for each
    // block of Native Asm code that correlates to one block of Wasm code.
    let bridgeWidth = 16;
    let watRect = watElem.getBoundingClientRect();
    let points = asmElems
      .map(elem => {
        let rect = elem.getBoundingClientRect();
        bridgeWidth = rect.left - watRect.width;
        return `0 ${watRect.y - 2}px, 100% ${rect.y - 2}px, 100% ${rect.bottom + 2}px, 0 ${watRect.bottom + 2}px`;
      })
      .join(",");

    // Perform the DOM modification inside an animation frame to give the browser a bit of
    // a breathing room.
    window.requestAnimationFrame(() => {
      // Change the bridging element styling: change the color to be consistent with
      // the Wasm offset, and use the points calculated above to give it a shape that
      // makes it look like it's bridging the left and right lists.
      let bridge = document.getElementById("bridge");
      bridge.style.display = "block";
      bridge.style.left = `${watRect.width}px`;
      bridge.style.width = `${bridgeWidth}px`;
      bridge.style.clipPath = `polygon(${points})`;
      bridge.style.backgroundColor = watElem.style.backgroundColor;

      // Draw a 2px dark outline in each block of instructions so it stands out a bit better
      // when hovered.
      let outline = `2px solid ${rgbToCss(rgbDarken(rgbForOffset(offset)))}`;
      for (const elem of elems) {
        // TODO: if any of these elems is out of view, show in the pop-up there it is (up or down)
        elem.setAttribute("title", `WASM offset @ ${offset}`);
        elem.classList.add("hovered");
        elem.style.outline = outline;
      }
    });
  });

  element.addEventListener("mouseleave", event => {
    document.getElementById("bridge").style.display = "none";
    eachElementWithSameWasmOff(event, elem => {
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
for (const func of window.ASM.functions) {
  const funcElem = document.createElement("div");

  const funcHeader = document.createElement("h3");
  let functionName =
    func.name === null ? `function[${func.func_index}]` : func.name;
  let demangledName =
    func.demangled_name !== null ? func.demangled_name : functionName;
  funcHeader.textContent = `Disassembly of function <${demangledName}>:`;
  funcHeader.title = `Function ${func.func_index}: ${functionName}`;
  funcElem.appendChild(funcHeader);

  let currentBlock = createDivForCode();
  let disasmBuffer = [];
  let lastOffset = null;

  const addCurrentBlock = offset => {
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
for (const chunk of window.WAT.chunks) {
  if (chunk.wasm_offset === null) continue;
  const block = createDivForCode();
  block.dataset.wasmOffset = chunk.wasm_offset;
  block.innerText = chunk.wat;

  if (offsetToRgb.get(chunk.wasm_offset) !== undefined) {
    adjustColorForOffset(block, chunk.wasm_offset);
    linkElements(block);
  }

  document.getElementById("wat").appendChild(block);
}
