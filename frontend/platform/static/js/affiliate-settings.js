document.addEventListener("DOMContentLoaded", () => {
    const form = document.getElementById("affiliate-settings-form");
    const saveBtn = document.getElementById("save-settings-btn");

    form.addEventListener("submit", async (e) => {
        e.preventDefault();
        saveBtn.disabled = true;
        saveBtn.innerHTML = 'Saving...';

        const formData = new FormData(form);
        const data = Object.fromEntries(formData.entries());

        try {
            // Note: We're calling the onboarding endpoint to update tax info for now
            // Future: specific patch endpoint for affiliate settings
            const res = await fetch("/api/affiliate/onboarding/submit", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(data)
            });

            if(!res.ok) {
                const errorData = await res.json();
                throw new Error(errorData.error || "Failed to update settings");
            }

            alert("Settings updated successfully! Your account will be briefly held pending tax verification.");
            window.location.reload();
        } catch (err) {
            console.error(err);
            alert(err.message);
        } finally {
            saveBtn.innerHTML = 'Save Changes';
            saveBtn.disabled = false;
        }
    });
});
