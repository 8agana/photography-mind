/**
 * Shared utilities for Sam Atagana Photography forms
 * Common functionality for competition and contact forms
 */

const FormUtils = {
    /**
     * Format phone number as user types
     * @param {HTMLInputElement} input - Phone input element
     */
    formatPhoneNumber(input) {
        input.addEventListener('input', function(e) {
            let value = e.target.value.replace(/\D/g, '');
            if (value.length > 0) {
                if (value.length <= 3) {
                    value = `(${value}`;
                } else if (value.length <= 6) {
                    value = `(${value.slice(0,3)}) ${value.slice(3)}`;
                } else {
                    value = `(${value.slice(0,3)}) ${value.slice(3,6)}-${value.slice(6,10)}`;
                }
            }
            e.target.value = value;
        });
    },

    /**
     * Save form data to localStorage
     * @param {string} formId - Form identifier for storage key
     * @param {HTMLFormElement} form - Form element
     */
    enableAutoSave(formId, form) {
        const storageKey = `samataganaphotography_${formId}`;
        
        // Save on input
        form.addEventListener('input', () => {
            const formData = new FormData(form);
            const data = Object.fromEntries(formData);
            localStorage.setItem(storageKey, JSON.stringify(data));
        });

        // Offer to restore on load
        const savedData = localStorage.getItem(storageKey);
        if (savedData) {
            const data = JSON.parse(savedData);
            const hasData = Object.values(data).some(val => val && val !== '');
            
            if (hasData && confirm('Would you like to continue where you left off?')) {
                FormUtils.restoreFormData(form, data);
            } else {
                localStorage.removeItem(storageKey);
            }
        }

        // Clear on successful submit
        form.addEventListener('submit', () => {
            setTimeout(() => {
                localStorage.removeItem(storageKey);
            }, 1000);
        });
    },

    /**
     * Restore form data from saved object
     * @param {HTMLFormElement} form - Form element
     * @param {Object} data - Saved form data
     */
    restoreFormData(form, data) {
        Object.entries(data).forEach(([name, value]) => {
            const field = form.querySelector(`[name="${name}"]`);
            if (field) {
                if (field.type === 'checkbox') {
                    field.checked = value === 'on' || value === true;
                    // Trigger change event for checkbox logic
                    field.dispatchEvent(new Event('change'));
                } else {
                    field.value = value;
                    // Trigger change for dropdowns
                    if (field.tagName === 'SELECT') {
                        field.dispatchEvent(new Event('change'));
                    }
                }
            }
        });
    },

    /**
     * Validate email format
     * @param {string} email - Email address
     * @returns {boolean} - True if valid
     */
    isValidEmail(email) {
        return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
    },

    /**
     * Validate phone format (10 digits)
     * @param {string} phone - Phone number
     * @returns {boolean} - True if valid
     */
    isValidPhone(phone) {
        const digitsOnly = phone.replace(/\D/g, '');
        return digitsOnly.length === 10;
    },

    /**
     * Show error message for a field
     * @param {HTMLElement} field - Input field
     * @param {string} message - Error message
     */
    showError(field, message) {
        const errorElement = document.getElementById(`${field.id}-error`);
        if (errorElement) {
            errorElement.textContent = message;
            errorElement.style.display = 'block';
        }
        field.classList.add('error');
    },

    /**
     * Hide error message for a field
     * @param {HTMLElement} field - Input field
     */
    hideError(field) {
        const errorElement = document.getElementById(`${field.id}-error`);
        if (errorElement) {
            errorElement.style.display = 'none';
        }
        field.classList.remove('error');
    },

    /**
     * Handle form submission with loading state
     * @param {HTMLFormElement} form - Form element
     * @param {HTMLButtonElement} button - Submit button
     * @param {string} scriptURL - Google Apps Script URL
     * @param {string} redirectURL - Success redirect URL
     */
    handleFormSubmit(form, button, scriptURL, redirectURL) {
        form.addEventListener('submit', async function(event) {
            event.preventDefault();

            // Honeypot check
            const honeypot = form.querySelector('input[name="website"]');
            if (honeypot && honeypot.value) {
                console.log('Honeypot triggered');
                return;
            }

            // Disable button and show loading
            const originalText = button.textContent;
            button.disabled = true;
            button.textContent = 'Submitting...';

            try {
                const formData = new FormData(form);
                await fetch(scriptURL, {
                    method: 'POST',
                    body: formData,
                    mode: 'no-cors'
                });

                // Success - redirect
                window.location.href = redirectURL;
            } catch (error) {
                console.error('Submission error:', error);
                alert('Error submitting form. Please check your connection and try again.');
                
                // Reset button
                button.disabled = false;
                button.textContent = originalText;
            }
        });
    },

    /**
     * Load competition data from external source
     * @param {string} url - URL to competitions JSON
     * @param {HTMLSelectElement} selectElement - Dropdown to populate
     */
    async loadCompetitions(url, selectElement) {
        try {
            const response = await fetch(url);
            const competitions = await response.json();
            
            // Clear existing options except the placeholder
            selectElement.innerHTML = '<option value="" disabled selected>Select an option</option>';
            
            // Add competitions
            competitions.forEach(comp => {
                const option = document.createElement('option');
                option.value = comp.name;
                option.textContent = comp.dates ? `${comp.name} - ${comp.dates}` : comp.name;
                selectElement.appendChild(option);
            });
        } catch (error) {
            console.error('Error loading competitions:', error);
            // Fallback to hardcoded if needed
        }
    },

    /**
     * Remember user info for future forms
     * @param {Object} userInfo - User information to save
     */
    saveUserInfo(userInfo) {
        const storageKey = 'samataganaphotography_user';
        localStorage.setItem(storageKey, JSON.stringify(userInfo));
    },

    /**
     * Get saved user info
     * @returns {Object|null} - Saved user info or null
     */
    getSavedUserInfo() {
        const storageKey = 'samataganaphotography_user';
        const saved = localStorage.getItem(storageKey);
        return saved ? JSON.parse(saved) : null;
    },

    /**
     * Auto-fill user fields if info is saved
     * @param {HTMLFormElement} form - Form to auto-fill
     */
    autoFillUserInfo(form) {
        const userInfo = this.getSavedUserInfo();
        if (userInfo) {
            ['firstName', 'lastName', 'email', 'phone'].forEach(field => {
                const input = form.querySelector(`[name*="${field}"]`);
                if (input && userInfo[field]) {
                    input.value = userInfo[field];
                }
            });
        }
    }
};

// Add CSS for error state
const style = document.createElement('style');
style.textContent = `
    .form-input.error,
    .form-textarea.error,
    .form-dropdown.error {
        border-color: #ff4444;
    }
`;
document.head.appendChild(style);

// Export for use in forms
window.FormUtils = FormUtils;