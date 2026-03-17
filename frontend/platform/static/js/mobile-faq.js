// Mobile FAQ Accordion

document.addEventListener("DOMContentLoaded", function () {
  const faqItems = document.querySelectorAll(".mobile-faq-item");

  faqItems.forEach((item) => {
    const header = item.querySelector(".mobile-faq-item-header");

    header.addEventListener("click", function () {
      // Check if this item is already active
      const isActive = item.classList.contains("active");

      // Close all FAQ items
      faqItems.forEach((faq) => faq.classList.remove("active"));

      // If it wasn't active, open it
      if (!isActive) {
        item.classList.add("active");
      }
    });
  });
});
