export interface SettingsResponse {
    email: string;
    first_name: string | null;
    last_name: string | null;
    phone_number: string | null;
    country: string | null;
    timezone: string;
    role: string;
    language: string;
    currency: string;
}

export interface ApiResponse {
    success: boolean;
    message: string;
}

export interface UpdateProfileForm {
    first_name: string | null;
    last_name: string | null;
    phone_number: string | null;
    country: string | null;
    timezone: string | null;
    profile_photo?: string | null;
}

export interface UpdatePreferencesForm {
    language: string;
    currency: string;
}

export interface ChangeEmailForm {
    new_email: string;
    current_password?: string;
}

export interface ChangePasswordForm {
    current_password?: string;
    new_password?: string;
    confirm_password?: string;
}

export interface ChangePhoneForm {
    new_phone: string;
}
