import {
  FilesetResolver,
  HandLandmarker,
} from "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.18";

const VIDEO_WIDTH = 1280;
const VIDEO_HEIGHT = 720;
const MAX_DRONES = 8;
const PUSH_COOLDOWN_MS = 1500;
const PUSH_HISTORY_SIZE = 6;
const PUSH_AREA_DELTA = 0.035;
const PUSH_Z_DELTA = 0.09;
const BLADE_SCALE = 240;
const COMBO_WINDOW_MS = 2200;

const startButton = document.getElementById("start-button");
const video = document.getElementById("camera");
const canvas = document.getElementById("arena-canvas");
const ctx = canvas.getContext("2d");

const runtimeBadge = document.getElementById("runtime-badge");
const trackingBadge = document.getElementById("tracking-badge");
const heroStatus = document.getElementById("hero-status");
const forceStatus = document.getElementById("force-status");
const saberStatus = document.getElementById("saber-status");
const fpsStatus = document.getElementById("fps-status");
const scoreValue = document.getElementById("score-value");
const comboValue = document.getElementById("combo-value");
const levelValue = document.getElementById("level-value");
const droneCount = document.getElementById("drone-count");
const cooldownValue = document.getElementById("cooldown-value");
const audioValue = document.getElementById("audio-value");

const state = {
  landmarker: null,
  stream: null,
  running: false,
  videoReady: false,
  lastVideoTime: -1,
  fpsSamples: [],
  lastFrameTimestamp: 0,
  score: 0,
  combo: 0,
  comboMultiplier: 1,
  comboExpiresAt: 0,
  difficultyLevel: 1,
  danger: 0,
  drones: [],
  particles: [],
  sparks: [],
  pushHistory: [],
  lastPushAt: 0,
  pushPulseUntil: 0,
  pushState: "Idle",
  saber: null,
  forceHand: null,
  saberHand: null,
  audio: {
    context: null,
    master: null,
    armed: false,
  },
  roleMemory: {
    saberId: null,
    forceId: null,
  },
  heroMessageUntil: 0,
};

startButton.addEventListener("click", startExperience);
window.addEventListener("resize", resizeCanvas);

resizeCanvas();
seedDrones();
animate(performance.now());

async function startExperience() {
  if (state.running) {
    return;
  }

  startButton.disabled = true;
  startButton.textContent = "Starting...";
  setRuntime("Loading vision stack", false);

  try {
    await setupAudio();
    await setupCamera();
    await setupHandTracking();
    state.running = true;
    setRuntime("Live", true);
    trackingBadge.textContent = "Tracking Online";
    trackingBadge.classList.remove("status-badge-muted");
    setHeroMessage("Camera active. Raise both hands in frame.", 2200);
    startButton.textContent = "Camera Running";
    audioValue.textContent = state.audio.armed ? "Live" : "Muted";
    playTone(220, 0.06, "triangle", 0.05);
  } catch (error) {
    console.error(error);
    setRuntime("Camera blocked", false);
    trackingBadge.textContent = "Tracking Offline";
    trackingBadge.classList.add("status-badge-muted");
    heroStatus.textContent = "Unable to start. Serve locally and allow camera permissions.";
    startButton.disabled = false;
    startButton.textContent = "Retry Camera";
    audioValue.textContent = "Standby";
  }
}

async function setupAudio() {
  const AudioContextClass = window.AudioContext || window.webkitAudioContext;
  if (!AudioContextClass) {
    audioValue.textContent = "Unsupported";
    return;
  }

  if (!state.audio.context) {
    state.audio.context = new AudioContextClass();
    state.audio.master = state.audio.context.createGain();
    state.audio.master.gain.value = 0.08;
    state.audio.master.connect(state.audio.context.destination);
  }

  if (state.audio.context.state === "suspended") {
    await state.audio.context.resume();
  }

  state.audio.armed = state.audio.context.state === "running";
}

async function setupCamera() {
  const stream = await navigator.mediaDevices.getUserMedia({
    video: {
      width: { ideal: VIDEO_WIDTH },
      height: { ideal: VIDEO_HEIGHT },
      facingMode: "user",
    },
    audio: false,
  });

  state.stream = stream;
  video.srcObject = stream;

  await new Promise((resolve) => {
    video.onloadedmetadata = () => {
      video.play();
      state.videoReady = true;
      resizeCanvas();
      resolve();
    };
  });
}

async function setupHandTracking() {
  const vision = await FilesetResolver.forVisionTasks(
    "https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.18/wasm"
  );

  state.landmarker = await HandLandmarker.createFromOptions(vision, {
    baseOptions: {
      modelAssetPath:
        "https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/1/hand_landmarker.task",
    },
    numHands: 2,
    runningMode: "VIDEO",
    minHandDetectionConfidence: 0.45,
    minHandPresenceConfidence: 0.45,
    minTrackingConfidence: 0.45,
  });
}

function resizeCanvas() {
  const rect = canvas.getBoundingClientRect();
  const ratio = window.devicePixelRatio || 1;
  canvas.width = Math.round(rect.width * ratio);
  canvas.height = Math.round(rect.height * ratio);
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
}

function animate(timestamp) {
  const dt = Math.min((timestamp - state.lastFrameTimestamp) / 1000 || 0.016, 0.033);
  state.lastFrameTimestamp = timestamp;

  updateFps(timestamp);
  updateVision(timestamp);
  updateGame(dt, timestamp);
  render(timestamp);

  requestAnimationFrame(animate);
}

function updateFps(timestamp) {
  state.fpsSamples.push(timestamp);
  while (state.fpsSamples.length && state.fpsSamples[0] < timestamp - 1000) {
    state.fpsSamples.shift();
  }
  fpsStatus.textContent = `${state.fpsSamples.length} FPS`;
}

function updateVision(timestamp) {
  if (!state.running || !state.videoReady || !state.landmarker) {
    return;
  }

  if (video.currentTime === state.lastVideoTime) {
    return;
  }

  state.lastVideoTime = video.currentTime;

  const result = state.landmarker.detectForVideo(video, timestamp);
  const hands = buildHands(result);
  const assigned = assignHandRoles(hands);

  state.saberHand = assigned.saberHand;
  state.forceHand = assigned.forceHand;
  state.saber = state.saberHand ? createSaberFromHand(state.saberHand) : null;

  if (state.saberHand) {
    saberStatus.textContent = `${state.saberHand.label} lock`;
  } else {
    saberStatus.textContent = "Searching";
  }

  evaluatePush(timestamp);
}

function buildHands(result) {
  if (!result?.landmarks) {
    return [];
  }

  return result.landmarks.map((landmarks, index) => {
    const handedness = result.handednesses?.[index]?.[0]?.categoryName || "Unknown";
    const screenLandmarks = landmarks.map((point) => ({
      x: (1 - point.x) * canvas.clientWidth,
      y: point.y * canvas.clientHeight,
      z: point.z,
    }));

    const bounds = screenLandmarks.reduce(
      (acc, point) => {
        acc.minX = Math.min(acc.minX, point.x);
        acc.minY = Math.min(acc.minY, point.y);
        acc.maxX = Math.max(acc.maxX, point.x);
        acc.maxY = Math.max(acc.maxY, point.y);
        return acc;
      },
      { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity }
    );

    const width = Math.max(1, bounds.maxX - bounds.minX);
    const height = Math.max(1, bounds.maxY - bounds.minY);
    const thumb = screenLandmarks[4];
    const pinky = screenLandmarks[20];

    return {
      id: `${handedness}-${Math.round((bounds.minX + bounds.maxX) / 24)}-${Math.round((bounds.minY + bounds.maxY) / 24)}`,
      handedness,
      label: handedness === "Left" ? "Right-side" : handedness === "Right" ? "Left-side" : "Unknown",
      landmarks: screenLandmarks,
      center: {
        x: (bounds.minX + bounds.maxX) / 2,
        y: (bounds.minY + bounds.maxY) / 2,
      },
      openness: Math.hypot(thumb.x - pinky.x, thumb.y - pinky.y),
      area: (width * height) / Math.max(canvas.clientWidth * canvas.clientHeight, 1),
    };
  });
}

function assignHandRoles(hands) {
  if (!hands.length) {
    state.roleMemory.saberId = null;
    state.roleMemory.forceId = null;
    return { saberHand: null, forceHand: null };
  }

  let saberHand = null;
  let forceHand = null;

  if (state.roleMemory.saberId) {
    saberHand = hands.find((hand) => hand.id === state.roleMemory.saberId) || null;
  }
  if (state.roleMemory.forceId) {
    forceHand = hands.find((hand) => hand.id === state.roleMemory.forceId) || null;
  }

  if (!saberHand) {
    saberHand = [...hands]
      .sort((a, b) => scoreSaberCandidate(b) - scoreSaberCandidate(a))[0] || null;
  }

  const remaining = hands.filter((hand) => hand !== saberHand);

  if (!forceHand) {
    forceHand = [...remaining]
      .sort((a, b) => scoreForceCandidate(b) - scoreForceCandidate(a))[0] || null;
  }

  if (!forceHand && hands.length === 1) {
    forceHand = null;
  }

  if (saberHand && forceHand && saberHand === forceHand) {
    forceHand = remaining[0] || null;
  }

  if (saberHand) {
    state.roleMemory.saberId = saberHand.id;
  }
  if (forceHand) {
    state.roleMemory.forceId = forceHand.id;
  }

  return { saberHand, forceHand };
}

function scoreSaberCandidate(hand) {
  let score = hand.center.x / Math.max(canvas.clientWidth, 1);
  if (hand.handedness === "Left") {
    score += 0.6;
  }
  score += hand.openness * 0.0008;
  return score;
}

function scoreForceCandidate(hand) {
  let score = (canvas.clientWidth - hand.center.x) / Math.max(canvas.clientWidth, 1);
  if (hand.handedness === "Right") {
    score += 0.6;
  }
  score += hand.area * 0.5;
  return score;
}

function createSaberFromHand(hand) {
  const wrist = hand.landmarks[0];
  const indexBase = hand.landmarks[5];
  const indexTip = hand.landmarks[8];

  const anchor = {
    x: wrist.x * 0.55 + indexBase.x * 0.45,
    y: wrist.y * 0.55 + indexBase.y * 0.45,
  };

  let dirX = indexTip.x - anchor.x;
  let dirY = indexTip.y - anchor.y;
  const length = Math.hypot(dirX, dirY) || 1;
  dirX /= length;
  dirY /= length;

  const handSize = Math.hypot(
    hand.landmarks[5].x - hand.landmarks[17].x,
    hand.landmarks[5].y - hand.landmarks[17].y
  );
  const bladeLength = Math.max(BLADE_SCALE * 0.65, Math.min(BLADE_SCALE * 1.24, handSize * 3.2));

  return {
    anchor,
    tip: {
      x: anchor.x + dirX * bladeLength,
      y: anchor.y + dirY * bladeLength,
    },
    guard: {
      x: anchor.x - dirX * 28,
      y: anchor.y - dirY * 28,
    },
    direction: { x: dirX, y: dirY },
    bladeLength,
  };
}

function evaluatePush(timestamp) {
  if (!state.forceHand) {
    state.pushHistory.length = 0;
    state.pushState = "Idle";
    forceStatus.textContent = "Idle";
    return;
  }

  const palm = state.forceHand.landmarks[0];
  const middleBase = state.forceHand.landmarks[9];
  const sample = {
    t: timestamp,
    area: state.forceHand.area,
    z: (palm.z + middleBase.z) / 2,
    x: state.forceHand.center.x,
    y: state.forceHand.center.y,
  };

  state.pushHistory.push(sample);
  if (state.pushHistory.length > PUSH_HISTORY_SIZE) {
    state.pushHistory.shift();
  }

  const oldest = state.pushHistory[0];
  const areaDelta = sample.area - oldest.area;
  const zDelta = oldest.z - sample.z;
  const cooldownRemaining = Math.max(0, PUSH_COOLDOWN_MS - (timestamp - state.lastPushAt));

  if (cooldownRemaining > 0) {
    state.pushState = `Cooldown ${Math.ceil(cooldownRemaining / 100) / 10}s`;
  } else if (areaDelta > PUSH_AREA_DELTA * 0.55 || zDelta > PUSH_Z_DELTA * 0.5) {
    state.pushState = "Charging";
  } else {
    state.pushState = "Idle";
  }

  if (cooldownRemaining <= 0 && areaDelta > PUSH_AREA_DELTA && zDelta > PUSH_Z_DELTA) {
    fireForcePush(sample, timestamp);
  }

  forceStatus.textContent = state.pushState;
  cooldownValue.textContent = cooldownRemaining > 0 ? `${(cooldownRemaining / 1000).toFixed(1)}s` : "Ready";
}

function fireForcePush(sample, timestamp) {
  state.lastPushAt = timestamp;
  state.pushPulseUntil = timestamp + 220;
  state.pushState = "Force Burst";
  forceStatus.textContent = "Force Burst";
  setHeroMessage("Push event fired", 900);
  playTone(180, 0.16, "sawtooth", 0.06);
  playTone(240, 0.18, "triangle", 0.04, 0.03);

  let affected = 0;
  let destroyed = 0;
  const radius = 180 + state.difficultyLevel * 10;

  for (const drone of state.drones) {
    if (!drone.active) {
      continue;
    }

    const dx = drone.x - sample.x;
    const dy = drone.y - sample.y;
    const distance = Math.hypot(dx, dy);

    if (distance < radius) {
      const strength = (radius - distance) / radius;
      const normX = dx / (distance || 1);
      const normY = dy / (distance || 1);
      drone.vx += normX * (280 + strength * 300);
      drone.vy += normY * (200 + strength * 220);
      drone.hitFlash = 0.9;
      affected += 1;

      if (distance < 120 + state.difficultyLevel * 4) {
        destroyDrone(drone, sample.x, sample.y, "push");
        destroyed += 1;
      }
    }
  }

  if (affected) {
    registerHit(Math.max(affected * 5, destroyed * 9), destroyed ? "Force Slam!" : "Force Wave!");
    spawnForceRing(sample.x, sample.y);
  } else {
    state.combo = Math.max(0, state.combo - 1);
  }
}

function updateGame(dt, timestamp) {
  if (state.combo > 0 && timestamp > state.comboExpiresAt) {
    state.combo = 0;
    state.comboMultiplier = 1;
  }

  updateDifficulty();

  for (const drone of state.drones) {
    if (!drone.active) {
      if (timestamp > drone.respawnAt) {
        resetDrone(drone);
      }
      continue;
    }

    drone.x += drone.vx * dt;
    drone.y += drone.vy * dt;
    drone.hitFlash = Math.max(0, drone.hitFlash - dt * 2.6);

    const centerX = canvas.clientWidth / 2;
    const centerY = canvas.clientHeight / 2;
    const toCenterX = centerX - drone.x;
    const toCenterY = centerY - drone.y;
    const distCenter = Math.hypot(toCenterX, toCenterY) || 1;
    drone.vx += (toCenterX / distCenter) * dt * (12 + state.difficultyLevel * 2.2);
    drone.vy += (toCenterY / distCenter) * dt * (12 + state.difficultyLevel * 2.2);

    if (
      state.saber &&
      distancePointToSegment(
        drone.x,
        drone.y,
        state.saber.guard.x,
        state.saber.guard.y,
        state.saber.tip.x,
        state.saber.tip.y
      ) < drone.radius + 10
    ) {
      destroyDrone(drone, drone.x, drone.y, "saber");
      registerHit(10 + state.difficultyLevel, "Deflect!");
    }

    if (distCenter < 44) {
      drone.active = false;
      drone.respawnAt = timestamp + 650;
      state.combo = 0;
      state.comboMultiplier = 1;
      state.danger = Math.min(1, state.danger + 0.18);
      setHeroMessage("Drone breach detected", 1200);
      playTone(120, 0.2, "square", 0.045);
      continue;
    }

    if (
      drone.x < -160 ||
      drone.x > canvas.clientWidth + 160 ||
      drone.y < -160 ||
      drone.y > canvas.clientHeight + 160
    ) {
      resetDrone(drone);
    }
  }

  for (const particle of state.particles) {
    particle.x += particle.vx * dt;
    particle.y += particle.vy * dt;
    particle.vx *= 0.98;
    particle.vy *= 0.98;
    particle.life -= dt;
  }
  state.particles = state.particles.filter((particle) => particle.life > 0);

  for (const spark of state.sparks) {
    spark.radius += spark.growth * dt;
    spark.life -= dt;
  }
  state.sparks = state.sparks.filter((spark) => spark.life > 0);

  state.danger = Math.max(0, state.danger - dt * 0.05);

  if (timestamp > state.heroMessageUntil && state.running) {
    heroStatus.textContent =
      state.comboMultiplier > 1
        ? `Combo x${state.comboMultiplier} active`
        : "Camera active. Deflect incoming drones.";
  }

  scoreValue.textContent = String(state.score);
  comboValue.textContent = `x${state.comboMultiplier}`;
  levelValue.textContent = String(state.difficultyLevel);
  droneCount.textContent = String(state.drones.filter((drone) => drone.active).length);
  audioValue.textContent = state.audio.armed ? "Live" : "Muted";
}

function updateDifficulty() {
  const computedLevel = 1 + Math.min(9, Math.floor(state.score / 140) + Math.floor(state.combo / 4));
  state.difficultyLevel = computedLevel;
}

function registerHit(basePoints, message) {
  state.combo += 1;
  state.comboMultiplier = Math.min(8, 1 + Math.floor(state.combo / 2));
  state.comboExpiresAt = performance.now() + COMBO_WINDOW_MS;
  state.score += basePoints * state.comboMultiplier;
  setHeroMessage(message, 850);
  state.danger = Math.max(0, state.danger - 0.08);
  playTone(280 + state.comboMultiplier * 20, 0.08, "triangle", 0.05);
  if (state.comboMultiplier >= 4) {
    playTone(420 + state.comboMultiplier * 16, 0.05, "sine", 0.03, 0.01);
  }
}

function destroyDrone(drone, x, y, source) {
  drone.active = false;
  const baseDelay = Math.max(380, 1050 - state.difficultyLevel * 65);
  drone.respawnAt = performance.now() + baseDelay + Math.random() * 600;
  spawnParticles(x, y, source === "push" ? "#9af7ff" : "#ffcb74");
}

function seedDrones() {
  for (let i = 0; i < MAX_DRONES; i += 1) {
    const drone = {
      x: 0,
      y: 0,
      vx: 0,
      vy: 0,
      radius: 0,
      active: true,
      hitFlash: 0,
      respawnAt: 0,
    };
    resetDrone(drone, true);
    state.drones.push(drone);
  }
}

function resetDrone(drone, initial = false) {
  const width = Math.max(canvas.clientWidth || window.innerWidth * 0.7, 640);
  const height = Math.max(canvas.clientHeight || window.innerHeight * 0.5, 420);
  const side = Math.floor(Math.random() * 4);
  const margin = 80;

  if (side === 0) {
    drone.x = -margin;
    drone.y = Math.random() * height;
  } else if (side === 1) {
    drone.x = width + margin;
    drone.y = Math.random() * height;
  } else if (side === 2) {
    drone.x = Math.random() * width;
    drone.y = -margin;
  } else {
    drone.x = Math.random() * width;
    drone.y = height + margin;
  }

  const centerX = width / 2 + (Math.random() - 0.5) * width * 0.12;
  const centerY = height / 2 + (Math.random() - 0.5) * height * 0.12;
  const dx = centerX - drone.x;
  const dy = centerY - drone.y;
  const distance = Math.hypot(dx, dy) || 1;
  const difficultySpeed = 72 + state.difficultyLevel * 12;
  const speed = difficultySpeed + Math.random() * 55 + (initial ? Math.random() * 16 : 0);

  drone.vx = (dx / distance) * speed;
  drone.vy = (dy / distance) * speed;
  drone.radius = Math.max(15, 24 - state.difficultyLevel * 0.55 + Math.random() * 14);
  drone.active = true;
  drone.hitFlash = 0;
}

function spawnParticles(x, y, color) {
  for (let i = 0; i < 18; i += 1) {
    const angle = (Math.PI * 2 * i) / 18 + Math.random() * 0.4;
    const speed = 60 + Math.random() * 180;
    state.particles.push({
      x,
      y,
      vx: Math.cos(angle) * speed,
      vy: Math.sin(angle) * speed,
      life: 0.35 + Math.random() * 0.45,
      size: 2 + Math.random() * 4,
      color,
    });
  }
}

function spawnForceRing(x, y) {
  state.sparks.push({
    x,
    y,
    radius: 24,
    growth: 280,
    life: 0.35,
  });
}

function render(timestamp) {
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;

  ctx.clearRect(0, 0, width, height);

  if (state.videoReady) {
    ctx.save();
    ctx.scale(-1, 1);
    ctx.drawImage(video, -width, 0, width, height);
    ctx.restore();
    ctx.fillStyle = `rgba(2, 6, 16, ${0.2 + state.danger * 0.15})`;
    ctx.fillRect(0, 0, width, height);
  } else {
    const gradient = ctx.createLinearGradient(0, 0, width, height);
    gradient.addColorStop(0, "#0d1b31");
    gradient.addColorStop(1, "#050914");
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, width, height);
  }

  drawGrid(width, height);
  drawCenterThreat(width, height);
  drawSparks();
  drawDrones(timestamp);
  drawParticles();
  drawHands();
  drawSaber(timestamp);
}

function drawGrid(width, height) {
  ctx.save();
  ctx.strokeStyle = "rgba(109, 187, 255, 0.12)";
  ctx.lineWidth = 1;
  for (let x = 0; x <= width; x += 48) {
    ctx.beginPath();
    ctx.moveTo(x, height * 0.56);
    ctx.lineTo(width / 2 + (x - width / 2) * 1.4, height);
    ctx.stroke();
  }
  for (let y = height * 0.56; y <= height; y += 34) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
    ctx.stroke();
  }
  ctx.restore();
}

function drawCenterThreat(width, height) {
  const radius = 36 + state.danger * 18;
  const gradient = ctx.createRadialGradient(width / 2, height / 2, 0, width / 2, height / 2, radius);
  gradient.addColorStop(0, `rgba(255, 110, 141, ${0.22 + state.danger * 0.18})`);
  gradient.addColorStop(1, "rgba(255, 110, 141, 0)");
  ctx.fillStyle = gradient;
  ctx.beginPath();
  ctx.arc(width / 2, height / 2, radius, 0, Math.PI * 2);
  ctx.fill();
}

function drawDrones(timestamp) {
  for (const drone of state.drones) {
    if (!drone.active) {
      continue;
    }

    const pulse = Math.sin(timestamp * 0.007 + drone.x * 0.01) * 0.5 + 0.5;
    const glowRadius = drone.radius + 18 + pulse * 6;

    const glow = ctx.createRadialGradient(drone.x, drone.y, 4, drone.x, drone.y, glowRadius);
    glow.addColorStop(0, "rgba(255,255,255,0.96)");
    glow.addColorStop(0.25, drone.hitFlash > 0 ? "rgba(255,173,120,0.92)" : "rgba(144,225,255,0.82)");
    glow.addColorStop(1, "rgba(23,62,94,0)");

    ctx.fillStyle = glow;
    ctx.beginPath();
    ctx.arc(drone.x, drone.y, glowRadius, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = drone.hitFlash > 0 ? "#ffd7a6" : "#79dfff";
    ctx.beginPath();
    ctx.arc(drone.x, drone.y, drone.radius, 0, Math.PI * 2);
    ctx.fill();

    ctx.strokeStyle = "rgba(255,255,255,0.55)";
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.arc(drone.x, drone.y, Math.max(6, drone.radius - 6), 0, Math.PI * 2);
    ctx.stroke();
  }
}

function drawParticles() {
  for (const particle of state.particles) {
    ctx.fillStyle = particle.color;
    ctx.globalAlpha = Math.max(0, particle.life);
    ctx.beginPath();
    ctx.arc(particle.x, particle.y, particle.size, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.globalAlpha = 1;
}

function drawSparks() {
  for (const spark of state.sparks) {
    ctx.strokeStyle = `rgba(150, 244, 255, ${spark.life * 1.8})`;
    ctx.lineWidth = 4;
    ctx.beginPath();
    ctx.arc(spark.x, spark.y, spark.radius, 0, Math.PI * 2);
    ctx.stroke();
  }
}

function drawHands() {
  const hands = [state.saberHand, state.forceHand].filter(Boolean);
  for (const hand of hands) {
    for (const point of hand.landmarks) {
      ctx.fillStyle = hand === state.saberHand ? "rgba(173, 233, 255, 0.82)" : "rgba(163, 255, 202, 0.82)";
      ctx.beginPath();
      ctx.arc(point.x, point.y, 4, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  if (state.forceHand) {
    const palm = state.forceHand.landmarks[0];
    ctx.strokeStyle = state.pushState.includes("Cooldown")
      ? "rgba(255, 189, 97, 0.75)"
      : "rgba(146, 255, 209, 0.75)";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(palm.x, palm.y, 88 + state.difficultyLevel * 2, 0, Math.PI * 2);
    ctx.stroke();
  }
}

function drawSaber(timestamp) {
  if (!state.saber) {
    return;
  }

  const flicker = 0.92 + Math.sin(timestamp * 0.02) * 0.08;
  const glow = ctx.createLinearGradient(
    state.saber.guard.x,
    state.saber.guard.y,
    state.saber.tip.x,
    state.saber.tip.y
  );
  glow.addColorStop(0, "rgba(90, 196, 255, 0.2)");
  glow.addColorStop(0.3, `rgba(116, 219, 255, ${0.58 * flicker})`);
  glow.addColorStop(1, `rgba(255, 255, 255, ${0.92 * flicker})`);

  ctx.strokeStyle = glow;
  ctx.lineWidth = 26;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(state.saber.anchor.x, state.saber.anchor.y);
  ctx.lineTo(state.saber.tip.x, state.saber.tip.y);
  ctx.stroke();

  ctx.strokeStyle = "#f7ffff";
  ctx.lineWidth = 7;
  ctx.beginPath();
  ctx.moveTo(state.saber.anchor.x, state.saber.anchor.y);
  ctx.lineTo(state.saber.tip.x, state.saber.tip.y);
  ctx.stroke();

  ctx.strokeStyle = "#8c97a8";
  ctx.lineWidth = 12;
  ctx.beginPath();
  ctx.moveTo(state.saber.guard.x, state.saber.guard.y);
  ctx.lineTo(state.saber.anchor.x, state.saber.anchor.y);
  ctx.stroke();

  if (state.pushPulseUntil > timestamp && state.forceHand) {
    const palm = state.forceHand.landmarks[0];
    ctx.fillStyle = "rgba(146, 255, 209, 0.18)";
    ctx.beginPath();
    ctx.arc(palm.x, palm.y, 112, 0, Math.PI * 2);
    ctx.fill();
  }
}

function distancePointToSegment(px, py, x1, y1, x2, y2) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const lengthSquared = dx * dx + dy * dy || 1;
  const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / lengthSquared));
  const cx = x1 + t * dx;
  const cy = y1 + t * dy;
  return Math.hypot(px - cx, py - cy);
}

function setRuntime(label, live) {
  runtimeBadge.textContent = label;
  runtimeBadge.classList.toggle("status-badge-muted", !live);
}

function setHeroMessage(message, durationMs) {
  heroStatus.textContent = message;
  state.heroMessageUntil = performance.now() + durationMs;
}

function playTone(frequency, duration, type = "sine", gainValue = 0.03, delay = 0) {
  if (!state.audio.armed || !state.audio.context || !state.audio.master) {
    return;
  }

  const now = state.audio.context.currentTime + delay;
  const oscillator = state.audio.context.createOscillator();
  const gain = state.audio.context.createGain();

  oscillator.type = type;
  oscillator.frequency.setValueAtTime(frequency, now);
  gain.gain.setValueAtTime(0.0001, now);
  gain.gain.exponentialRampToValueAtTime(gainValue, now + 0.01);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

  oscillator.connect(gain);
  gain.connect(state.audio.master);
  oscillator.start(now);
  oscillator.stop(now + duration + 0.03);
}
