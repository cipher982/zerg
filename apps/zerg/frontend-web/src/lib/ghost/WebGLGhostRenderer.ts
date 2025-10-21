/*
 * WebGLGhostRenderer
 *
 * A minimal WebGL2 overlay renderer that draws a textured quad at a given
 * screen-space position, sized in CSS pixels with DPR awareness. The texture
 * is produced from an offscreen 2D canvas (ghost skin) to match agent/tool nodes.
 */

export type GhostKind = "agent" | "tool";

export interface GhostSkinOptions {
  kind: GhostKind;
  label: string;
  icon: string; // emoji or small glyph
  width: number; // CSS px
  height: number; // CSS px
}

export interface GhostRenderState {
  x: number; // screen px (top-left)
  y: number; // screen px (top-left)
  opacity: number;
  scale: number; // multiplicative on size
}

const VERTEX_SHADER = `#version 300 es
precision highp float;

layout(location=0) in vec2 a_pos; // unit quad [0,1]

uniform vec2 u_resolution; // canvas size in physical pixels
uniform vec2 u_translation; // top-left in physical pixels
uniform vec2 u_size;        // size in physical pixels
uniform float u_dpr;        // device pixel ratio

out vec2 v_uv;

void main() {
  // a_pos is 0..1 in both axes
  vec2 px = u_translation + a_pos * u_size;
  // Convert to clip space. u_resolution is already in physical pixels.
  // Map px (0..res) => clip (-1..1). Flip Y because clip Y is up.
  vec2 clip = (px / u_resolution) * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  v_uv = a_pos;
}
`;

const FRAGMENT_SHADER = `#version 300 es
precision mediump float;

uniform sampler2D u_tex;
uniform float u_opacity;

in vec2 v_uv;
out vec4 outColor;

void main() {
  vec4 c = texture(u_tex, v_uv);
  outColor = vec4(c.rgb, c.a * u_opacity);
}
`;

function compileShader(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const sh = gl.createShader(type)!;
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(sh) || "";
    gl.deleteShader(sh);
    throw new Error("Shader compile error: " + log);
  }
  return sh;
}

function createProgram(gl: WebGL2RenderingContext, vsSrc: string, fsSrc: string): WebGLProgram {
  const vs = compileShader(gl, gl.VERTEX_SHADER, vsSrc);
  const fs = compileShader(gl, gl.FRAGMENT_SHADER, fsSrc);
  const prog = gl.createProgram()!;
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.linkProgram(prog);
  gl.deleteShader(vs);
  gl.deleteShader(fs);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(prog) || "";
    gl.deleteProgram(prog);
    throw new Error("Program link error: " + log);
  }
  return prog;
}

export class WebGLGhostRenderer {
  private canvas: HTMLCanvasElement;
  private gl: WebGL2RenderingContext;
  private program: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private tex: WebGLTexture | null = null;
  private dpr = 1;
  private running = false;
  private rafId: number | null = null;

  // uniforms
  private uResolution: WebGLUniformLocation;
  private uTranslation: WebGLUniformLocation;
  private uSize: WebGLUniformLocation;
  private uOpacity: WebGLUniformLocation;
  private uDpr: WebGLUniformLocation;

  // state
  private state: GhostRenderState = { x: 0, y: 0, opacity: 1, scale: 1 };
  private baseSize = { width: 160, height: 48 };

  constructor(parent: HTMLElement = document.body) {
    const canvas = document.createElement("canvas");
    canvas.style.position = "fixed"; // screen-space overlay
    canvas.style.left = "0";
    canvas.style.top = "0";
    canvas.style.width = "100vw";
    canvas.style.height = "100vh";
    canvas.style.pointerEvents = "none";
    canvas.style.zIndex = "210"; // above react-flow and previews
    parent.appendChild(canvas);

    this.canvas = canvas;

    const gl = canvas.getContext("webgl2", { premultipliedAlpha: true, alpha: true });
    if (!gl) throw new Error("WebGL2 not available");
    this.gl = gl;

    this.program = createProgram(gl, VERTEX_SHADER, FRAGMENT_SHADER);
    gl.useProgram(this.program);

    // Quad geometry (two triangles) in [0,1] unit space
    const verts = new Float32Array([
      0, 0,
      1, 0,
      0, 1,
      0, 1,
      1, 0,
      1, 1,
    ]);
    const vbo = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, verts, gl.STATIC_DRAW);

    const vao = gl.createVertexArray()!;
    gl.bindVertexArray(vao);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    this.vao = vao;

    this.uResolution = gl.getUniformLocation(this.program, "u_resolution")!;
    this.uTranslation = gl.getUniformLocation(this.program, "u_translation")!;
    this.uSize = gl.getUniformLocation(this.program, "u_size")!;
    this.uOpacity = gl.getUniformLocation(this.program, "u_opacity")!;
    this.uDpr = gl.getUniformLocation(this.program, "u_dpr")!;

    this.handleResize = this.handleResize.bind(this);
    window.addEventListener("resize", this.handleResize);
    this.handleResize();

    // Clear once
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  dispose() {
    this.stop();
    window.removeEventListener("resize", this.handleResize);
    try { this.gl.deleteTexture(this.tex); } catch {}
    try { this.gl.deleteVertexArray(this.vao); } catch {}
    try { this.gl.deleteProgram(this.program); } catch {}
    if (this.canvas.parentElement) this.canvas.parentElement.removeChild(this.canvas);
  }

  private handleResize() {
    const dpr = Math.max(1, Math.min(window.devicePixelRatio || 1, 3));
    this.dpr = dpr;
    const width = Math.floor(window.innerWidth * dpr);
    const height = Math.floor(window.innerHeight * dpr);
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
      this.gl.viewport(0, 0, width, height);
    }
  }

  prepareGhostTextureFromCanvas(source: HTMLCanvasElement) {
    const gl = this.gl;
    const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, source);
    this.tex = tex;
    this.baseSize = { width: source.width / this.dpr, height: source.height / this.dpr };
  }

  setGhostBaseSize(widthCssPx: number, heightCssPx: number) {
    this.baseSize = { width: widthCssPx, height: heightCssPx };
  }

  setState(next: Partial<GhostRenderState>) {
    this.state = { ...this.state, ...next };
  }

  start() {
    if (this.running) return;
    this.running = true;
    const loop = () => {
      if (!this.running) return;
      this.drawFrame();
      this.rafId = requestAnimationFrame(loop);
    };
    this.rafId = requestAnimationFrame(loop);
  }

  stop() {
    this.running = false;
    if (this.rafId != null) cancelAnimationFrame(this.rafId);
    this.rafId = null;
    // clear overlay
    const gl = this.gl;
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  private drawFrame() {
    const gl = this.gl;
    if (!this.tex) {
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      return;
    }

    gl.useProgram(this.program);
    gl.bindVertexArray(this.vao);

    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.tex);

    const resX = this.canvas.width; // physical px
    const resY = this.canvas.height;

    gl.uniform2f(this.uResolution, resX, resY);
    gl.uniform1f(this.uDpr, this.dpr);

    const sizeX = Math.max(1, Math.round(this.baseSize.width * this.state.scale * this.dpr));
    const sizeY = Math.max(1, Math.round(this.baseSize.height * this.state.scale * this.dpr));
    const transX = Math.round(this.state.x * this.dpr);
    const transY = Math.round(this.state.y * this.dpr);

    gl.uniform2f(this.uSize, sizeX, sizeY);
    gl.uniform2f(this.uTranslation, transX, transY);
    gl.uniform1f(this.uOpacity, this.state.opacity);

    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  animateDropTo(targetX: number, targetY: number, targetScale: number, durationMs = 200): Promise<void> {
    const start = performance.now();
    const startX = this.state.x;
    const startY = this.state.y;
    const startScale = this.state.scale;

    return new Promise((resolve) => {
      const step = () => {
        const t = Math.min(1, (performance.now() - start) / durationMs);
        // Minimum-jerk easing
        const s = t * t * t * (t * (6.0 * t - 15.0) + 10.0);
        this.setState({
          x: startX + (targetX - startX) * s,
          y: startY + (targetY - startY) * s,
          scale: startScale + (targetScale - startScale) * s,
          opacity: 1,
        });
        if (t < 1) {
          requestAnimationFrame(step);
        } else {
          resolve();
        }
      };
      requestAnimationFrame(step);
    });
  }
}

export function createGhostSkinCanvas(opts: GhostSkinOptions): HTMLCanvasElement {
  const dpr = Math.max(1, Math.min(window.devicePixelRatio || 1, 3));
  const w = Math.max(32, Math.round(opts.width));
  const h = Math.max(24, Math.round(opts.height));
  const cw = Math.round(w * dpr);
  const ch = Math.round(h * dpr);
  const canvas = document.createElement("canvas");
  canvas.width = cw;
  canvas.height = ch;
  const ctx = canvas.getContext("2d")!;
  ctx.scale(dpr, dpr);

  // Background
  if (opts.kind === "agent") {
    const g = ctx.createLinearGradient(0, 0, 0, h);
    g.addColorStop(0, "#2e2e3e");
    g.addColorStop(1, "#2a2a3a");
    ctx.fillStyle = g;
    roundRect(ctx, 0.5, 0.5, w - 1, h - 1, 8);
    ctx.fill();
    ctx.strokeStyle = "#3d3d5c";
    ctx.lineWidth = 1;
    roundRect(ctx, 0.5, 0.5, w - 1, h - 1, 8);
    ctx.stroke();
    ctx.fillStyle = "#e0e0e0";
  } else {
    const g = ctx.createLinearGradient(0, 0, 0, h);
    g.addColorStop(0, "#f8fafc");
    g.addColorStop(1, "#f1f5f9");
    ctx.fillStyle = g;
    roundRect(ctx, 0.5, 0.5, w - 1, h - 1, 6);
    ctx.fill();
    ctx.strokeStyle = "#e2e8f0";
    ctx.lineWidth = 1;
    roundRect(ctx, 0.5, 0.5, w - 1, h - 1, 6);
    ctx.stroke();
    ctx.fillStyle = "#1e293b";
  }

  // Icon
  ctx.font = "16px system-ui, -apple-system, sans-serif";
  ctx.textBaseline = "middle";
  ctx.textAlign = "left";
  ctx.fillText(opts.icon, 12, h / 2);

  // Label
  ctx.font = "600 14px system-ui, -apple-system, sans-serif";
  ctx.fillText(opts.label, 36, h / 2);

  return canvas;
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  const rr = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + rr, y);
  ctx.lineTo(x + w - rr, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + rr);
  ctx.lineTo(x + w, y + h - rr);
  ctx.quadraticCurveTo(x + w, y + h, x + w - rr, y + h);
  ctx.lineTo(x + rr, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - rr);
  ctx.lineTo(x, y + rr);
  ctx.quadraticCurveTo(x, y, x + rr, y);
}
