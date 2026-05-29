(function () {
  var stories = [
    {
      image: "/static/images/auth/generated/customer-ardi.webp",
      alt: "Young POOOL customer portrait in a calm workspace",
      quote: "I always know what I am signing into: my wallet, my assets, and the next step.",
      name: "Ardi Pratama",
      position: "68% center"
    },
    {
      image: "/static/images/auth/generated/customer-maya.webp",
      alt: "POOOL customer portrait in a modern apartment workspace",
      quote: "POOOL makes ownership feel simple without making it feel casual.",
      name: "Maya Wijaya",
      position: "72% center"
    },
    {
      image: "/static/images/auth/generated/customer-david.webp",
      alt: "POOOL customer portrait in a premium fintech lounge",
      quote: "The account view gives me a clear place to continue, review, and decide.",
      name: "David Tan",
      position: "66% center"
    }
  ];

  function initCarousel(carousel) {
    var image = carousel.querySelector("[data-auth-carousel-image]");
    var quote = carousel.querySelector("[data-auth-carousel-quote]");
    var name = carousel.querySelector("[data-auth-carousel-name]");
    var dots = Array.prototype.slice.call(carousel.querySelectorAll("[data-auth-carousel-dot]"));
    var prev = carousel.querySelector("[data-auth-carousel-prev]");
    var next = carousel.querySelector("[data-auth-carousel-next]");
    var current = 0;
    var intervalMs = 8500;
    var timer = null;

    function showStory(index) {
      current = (index + stories.length) % stories.length;
      var story = stories[current];
      image.src = story.image;
      image.alt = story.alt;
      image.style.objectPosition = story.position;
      quote.textContent = "\"" + story.quote + "\"";
      name.textContent = story.name;
      dots.forEach(function (dot, dotIndex) {
        dot.classList.toggle("is-active", dotIndex === current);
      });
    }

    function startTimer() {
      window.clearInterval(timer);
      timer = window.setInterval(function () {
        showStory(current + 1);
      }, intervalMs);
    }

    function moveTo(index) {
      showStory(index);
      startTimer();
    }

    if (prev) {
      prev.addEventListener("click", function () {
        moveTo(current - 1);
      });
    }

    if (next) {
      next.addEventListener("click", function () {
        moveTo(current + 1);
      });
    }

    dots.forEach(function (dot, dotIndex) {
      dot.addEventListener("click", function () {
        moveTo(dotIndex);
      });
    });

    showStory(0);
    startTimer();
  }

  document.addEventListener("DOMContentLoaded", function () {
    Array.prototype.forEach.call(document.querySelectorAll("[data-auth-carousel]"), initCarousel);
  });
})();
