// Celebration Effects: Confetti + Fireworks for Submission Success Page

// ============================================
// Confetti Animation
// ============================================
function createConfetti() {
  const confettiContainer = document.querySelector(".confetti-container");
  if (!confettiContainer) return;

  const colors = [
    "#0000FF",
    "#98FB96",
    "#FFD700",
    "#FF6B6B",
    "#4ECDC4",
    "#FF69B4",
    "#00CED1",
    "#FFA500",
  ];
  const confettiCount = 50;

  for (let i = 0; i < confettiCount; i++) {
    const confetti = document.createElement("div");
    confetti.className = "confetti-piece";

    // Random properties
    const leftPosition = Math.random() * 100;
    const animationDelay = Math.random() * 2;
    const animationDuration = 2 + Math.random() * 2;
    const rotation = Math.random() * 360;
    const size = 8 + Math.random() * 6;
    const color = colors[Math.floor(Math.random() * colors.length)];

    confetti.style.left = `${leftPosition}%`;
    confetti.style.width = `${size}px`;
    confetti.style.height = `${size}px`;
    confetti.style.backgroundColor = color;
    confetti.style.animationDelay = `${animationDelay}s`;
    confetti.style.animationDuration = `${animationDuration}s`;
    confetti.style.transform = `rotate(${rotation}deg)`;

    confettiContainer.appendChild(confetti);
  }

  // Remove confetti after animation
  setTimeout(() => {
    confettiContainer.style.display = "none";
  }, 5000);
}

// ============================================
// Fireworks Animation (Canvas-based)
// ============================================
function createFireworks() {
  // Create canvas for fireworks
  const canvas = document.createElement("canvas");
  canvas.id = "fireworks-canvas";
  canvas.style.position = "fixed";
  canvas.style.top = "0";
  canvas.style.left = "0";
  canvas.style.width = "100vw";
  canvas.style.height = "100vh";
  canvas.style.pointerEvents = "none";
  canvas.style.zIndex = "999";
  document.body.appendChild(canvas);

  const ctx = canvas.getContext("2d");
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;

  const particles = [];
  const colors = [
    "#0000FF",
    "#98FB96",
    "#FFD700",
    "#FF6B6B",
    "#4ECDC4",
    "#FF69B4",
    "#00CED1",
    "#FFA500",
  ];

  class Particle {
    constructor(x, y, color) {
      this.x = x;
      this.y = y;
      this.color = color;
      this.velocity = {
        x: (Math.random() - 0.5) * 8,
        y: (Math.random() - 0.5) * 8,
      };
      this.alpha = 1;
      this.decay = 0.015;
      this.size = 2 + Math.random() * 3;
    }

    draw() {
      ctx.save();
      ctx.globalAlpha = this.alpha;
      ctx.fillStyle = this.color;
      ctx.beginPath();
      ctx.arc(this.x, this.y, this.size, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    update() {
      this.velocity.y += 0.1; // gravity
      this.x += this.velocity.x;
      this.y += this.velocity.y;
      this.alpha -= this.decay;
    }
  }

  function createFirework(x, y) {
    const particleCount = 30 + Math.floor(Math.random() * 20);
    const color = colors[Math.floor(Math.random() * colors.length)];

    for (let i = 0; i < particleCount; i++) {
      particles.push(new Particle(x, y, color));
    }
  }

  function animate() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    particles.forEach((particle, index) => {
      if (particle.alpha <= 0) {
        particles.splice(index, 1);
      } else {
        particle.update();
        particle.draw();
      }
    });

    if (particles.length > 0) {
      requestAnimationFrame(animate);
    } else {
      // Remove canvas when done
      canvas.remove();
    }
  }

  // Launch fireworks at intervals
  let fireworkCount = 0;
  const maxFireworks = 8;

  const fireworkInterval = setInterval(() => {
    if (fireworkCount >= maxFireworks) {
      clearInterval(fireworkInterval);
      return;
    }

    const x = canvas.width * (0.2 + Math.random() * 0.6);
    const y = canvas.height * (0.2 + Math.random() * 0.4);
    createFirework(x, y);
    fireworkCount++;

    if (fireworkCount === 1) {
      animate();
    }
  }, 400);
}

// ============================================
// Initialize on Page Load
// ============================================
document.addEventListener("DOMContentLoaded", function () {
  // Add confetti pieces dynamically
  createConfetti();

  // Launch fireworks after a short delay
  setTimeout(() => {
    createFireworks();
  }, 300);

  // Optional: Add card entrance animation
  const card = document.querySelector(".submission-card");
  if (card) {
    card.style.opacity = "0";
    card.style.transform = "scale(0.9) translateY(20px)";

    setTimeout(() => {
      card.style.transition = "all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1)";
      card.style.opacity = "1";
      card.style.transform = "scale(1) translateY(0)";
    }, 100);
  }
});

// ============================================
// Handle window resize
// ============================================
window.addEventListener("resize", function () {
  const canvas = document.getElementById("fireworks-canvas");
  if (canvas) {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
  }
});
