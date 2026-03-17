--
-- PostgreSQL database dump
--

\restrict 9dWWtz6hPYWzJmkp0Zo15A3JRVAwhMlncUUbGjn5sz04zeTiFygYEGEoBsoi7Tx

-- Dumped from database version 16.13 (Homebrew)
-- Dumped by pg_dump version 16.13 (Homebrew)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: trigger_set_updated_at(); Type: FUNCTION; Schema: public; Owner: martin
--

CREATE FUNCTION public.trigger_set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.trigger_set_updated_at() OWNER TO martin;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: admin_approval_requests; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.admin_approval_requests (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    requester_id uuid NOT NULL,
    approver_id uuid,
    action_type character varying(50) NOT NULL,
    entity_type character varying(50) NOT NULL,
    entity_id uuid,
    payload jsonb NOT NULL,
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    rejection_reason text,
    expires_at timestamp with time zone DEFAULT (now() + '24:00:00'::interval) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT admin_approval_requests_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'approved'::character varying, 'rejected'::character varying, 'expired'::character varying])::text[])))
);


ALTER TABLE public.admin_approval_requests OWNER TO martin;

--
-- Name: admin_invitations; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.admin_invitations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    email character varying(255) NOT NULL,
    role_id uuid NOT NULL,
    invited_by uuid NOT NULL,
    token_hash character varying(255) NOT NULL,
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    accepted_at timestamp with time zone,
    CONSTRAINT admin_invitations_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'accepted'::character varying, 'expired'::character varying, 'revoked'::character varying])::text[])))
);


ALTER TABLE public.admin_invitations OWNER TO martin;

--
-- Name: admin_permissions; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.admin_permissions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    role_id uuid NOT NULL,
    permission character varying(100) NOT NULL
);


ALTER TABLE public.admin_permissions OWNER TO martin;

--
-- Name: asset_documents; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.asset_documents (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    asset_id uuid NOT NULL,
    document_type character varying(50) NOT NULL,
    title character varying(255) NOT NULL,
    file_url character varying(512) NOT NULL,
    file_size_bytes bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT asset_documents_document_type_check CHECK (((document_type)::text = ANY ((ARRAY['proof_of_title'::character varying, 'legal_basis'::character varying, 'building_permit'::character varying, 'site_plan'::character varying, 'tax_npwp'::character varying, 'tax_pbb'::character varying, 'tax_bphtb'::character varying, 'license_nib'::character varying, 'id_card'::character varying, 'owner_npwp'::character varying, 'expose'::character varying, 'appraisal'::character varying, 'financial'::character varying, 'floor_plan'::character varying, 'other'::character varying])::text[])))
);


ALTER TABLE public.asset_documents OWNER TO martin;

--
-- Name: asset_financials; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.asset_financials (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    asset_id uuid NOT NULL,
    period_month integer NOT NULL,
    period_year integer NOT NULL,
    rental_income_cents bigint DEFAULT 0,
    appreciation_cents bigint DEFAULT 0,
    occupancy_rate_bps integer,
    expenses_cents bigint DEFAULT 0,
    net_income_cents bigint DEFAULT 0,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.asset_financials OWNER TO martin;

--
-- Name: asset_images; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.asset_images (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    asset_id uuid NOT NULL,
    image_url character varying(512) NOT NULL,
    alt_text character varying(255),
    sort_order integer DEFAULT 0 NOT NULL,
    is_cover boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.asset_images OWNER TO martin;

--
-- Name: asset_milestones; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.asset_milestones (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    asset_id uuid NOT NULL,
    title character varying(255) NOT NULL,
    description text,
    milestone_date timestamp with time zone,
    month_index integer,
    is_completed boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.asset_milestones OWNER TO martin;

--
-- Name: assets; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.assets (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    developer_user_id uuid,
    title character varying(255) NOT NULL,
    slug character varying(255) NOT NULL,
    short_description character varying(500),
    description text,
    asset_type character varying(30) NOT NULL,
    property_type character varying(50),
    area character varying(100),
    lease_type character varying(30),
    lease_term_years integer,
    land_size_sqm numeric(10,2),
    building_size_sqm numeric(10,2),
    bedrooms integer,
    bathrooms integer,
    construction_status character varying(50),
    year_built integer,
    location_city character varying(100),
    location_country character varying(3),
    location_address character varying(255),
    location_lat numeric(10,7),
    location_lng numeric(10,7),
    location_description text,
    google_maps_url character varying(512),
    video_url character varying(512),
    total_value_cents bigint NOT NULL,
    token_price_cents bigint NOT NULL,
    tokens_total integer NOT NULL,
    tokens_available integer NOT NULL,
    annual_yield_bps integer,
    capital_appreciation_bps integer,
    occupancy_rate_bps integer,
    operator_name character varying(255),
    term_months integer,
    fixed_roi_bps integer,
    revenue_min_cents bigint,
    revenue_max_cents bigint,
    expenses_cents bigint,
    net_profit_min_cents bigint,
    net_profit_max_cents bigint,
    investor_payout_cents bigint,
    operator_split_pct integer,
    poool_split_pct integer,
    funding_status character varying(30) DEFAULT 'upcoming'::character varying NOT NULL,
    featured boolean DEFAULT false NOT NULL,
    published boolean DEFAULT false NOT NULL,
    funding_start_at timestamp with time zone,
    funding_end_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT assets_asset_type_check CHECK (((asset_type)::text = ANY ((ARRAY['real_estate'::character varying, 'commercial_property'::character varying, 'commodity'::character varying, 'business'::character varying, 'startup'::character varying, 'land_plot'::character varying])::text[]))),
    CONSTRAINT assets_construction_status_check CHECK ((((construction_status)::text = ANY ((ARRAY['ready'::character varying, 'construction'::character varying, 'renovation'::character varying])::text[])) OR (construction_status IS NULL))),
    CONSTRAINT assets_funding_status_check CHECK (((funding_status)::text = ANY ((ARRAY['upcoming'::character varying, 'funding_open'::character varying, 'funding_in_progress'::character varying, 'funded'::character varying, 'rented'::character varying, 'payout_pending'::character varying, 'exited'::character varying])::text[]))),
    CONSTRAINT assets_lease_type_check CHECK ((((lease_type)::text = ANY ((ARRAY['leasehold'::character varying, 'freehold'::character varying])::text[])) OR (lease_type IS NULL)))
);


ALTER TABLE public.assets OWNER TO martin;

--
-- Name: audit_logs; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.audit_logs (
    id bigint NOT NULL,
    actor_user_id uuid,
    action character varying(100) NOT NULL,
    entity_type character varying(50) NOT NULL,
    entity_id uuid,
    previous_state jsonb,
    new_state jsonb,
    ip_address inet,
    user_agent text,
    metadata jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.audit_logs OWNER TO martin;

--
-- Name: audit_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: martin
--

CREATE SEQUENCE public.audit_logs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_logs_id_seq OWNER TO martin;

--
-- Name: audit_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: martin
--

ALTER SEQUENCE public.audit_logs_id_seq OWNED BY public.audit_logs.id;


--
-- Name: cart_items; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.cart_items (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    tokens_quantity integer DEFAULT 1 NOT NULL,
    token_price_cents bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT cart_items_tokens_quantity_check CHECK ((tokens_quantity > 0))
);


ALTER TABLE public.cart_items OWNER TO martin;

--
-- Name: deposit_requests; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.deposit_requests (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    currency character varying(3) NOT NULL,
    amount_cents bigint NOT NULL,
    provider character varying(30) NOT NULL,
    provider_reference character varying(255),
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    payment_method character varying(50),
    expires_at timestamp with time zone,
    paid_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT deposit_requests_amount_cents_check CHECK ((amount_cents > 0)),
    CONSTRAINT deposit_requests_provider_check CHECK (((provider)::text = ANY ((ARRAY['stripe'::character varying, 'xendit'::character varying, 'midtrans'::character varying, 'mangopay'::character varying, 'manual'::character varying])::text[]))),
    CONSTRAINT deposit_requests_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'paid'::character varying, 'expired'::character varying, 'failed'::character varying, 'cancelled'::character varying])::text[])))
);


ALTER TABLE public.deposit_requests OWNER TO martin;

--
-- Name: developer_projects; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.developer_projects (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    developer_id uuid NOT NULL,
    asset_id uuid,
    project_name character varying(255) NOT NULL,
    status character varying(20) DEFAULT 'draft'::character varying NOT NULL,
    total_raised_cents bigint DEFAULT 0,
    investors_count integer DEFAULT 0,
    funding_progress_bps integer DEFAULT 0,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT developer_projects_status_check CHECK (((status)::text = ANY ((ARRAY['draft'::character varying, 'submitted'::character varying, 'in_review'::character varying, 'approved'::character varying, 'rejected'::character varying, 'live'::character varying])::text[])))
);


ALTER TABLE public.developer_projects OWNER TO martin;

--
-- Name: dividend_payouts; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.dividend_payouts (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    investment_id uuid NOT NULL,
    user_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    amount_cents bigint NOT NULL,
    payout_type character varying(20) NOT NULL,
    status character varying(20) DEFAULT 'scheduled'::character varying NOT NULL,
    scheduled_at timestamp with time zone,
    paid_at timestamp with time zone,
    wallet_tx_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT dividend_payouts_payout_type_check CHECK (((payout_type)::text = ANY ((ARRAY['rental'::character varying, 'exit'::character varying, 'bonus'::character varying])::text[]))),
    CONSTRAINT dividend_payouts_status_check CHECK (((status)::text = ANY ((ARRAY['scheduled'::character varying, 'processing'::character varying, 'paid'::character varying, 'failed'::character varying])::text[])))
);


ALTER TABLE public.dividend_payouts OWNER TO martin;

--
-- Name: email_logs; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.email_logs (
    id bigint NOT NULL,
    user_id uuid,
    template_id uuid,
    subject character varying(255) NOT NULL,
    recipient_email character varying(255) NOT NULL,
    status character varying(50) DEFAULT 'sent'::character varying NOT NULL,
    provider_id character varying(255),
    error_message text,
    sent_at timestamp with time zone DEFAULT now() NOT NULL,
    delivered_at timestamp with time zone,
    opened_at timestamp with time zone,
    clicked_at timestamp with time zone,
    CONSTRAINT email_logs_status_check CHECK (((status)::text = ANY ((ARRAY['queued'::character varying, 'sent'::character varying, 'delivered'::character varying, 'opened'::character varying, 'clicked'::character varying, 'bounced'::character varying, 'failed'::character varying, 'spam_complaint'::character varying])::text[])))
);


ALTER TABLE public.email_logs OWNER TO martin;

--
-- Name: email_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: martin
--

CREATE SEQUENCE public.email_logs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.email_logs_id_seq OWNER TO martin;

--
-- Name: email_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: martin
--

ALTER SEQUENCE public.email_logs_id_seq OWNED BY public.email_logs.id;


--
-- Name: email_templates; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.email_templates (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name character varying(100) NOT NULL,
    subject character varying(255) NOT NULL,
    html_template text NOT NULL,
    text_template text,
    version integer DEFAULT 1 NOT NULL,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.email_templates OWNER TO martin;

--
-- Name: investment_limits; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.investment_limits (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    annual_limit_cents bigint DEFAULT 25000000 NOT NULL,
    invested_12m_cents bigint DEFAULT 0 NOT NULL,
    available_cents bigint GENERATED ALWAYS AS ((annual_limit_cents - invested_12m_cents)) STORED,
    limit_year integer NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.investment_limits OWNER TO martin;

--
-- Name: investments; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.investments (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    tokens_owned integer DEFAULT 0 NOT NULL,
    purchase_value_cents bigint NOT NULL,
    current_value_cents bigint NOT NULL,
    total_rental_cents bigint DEFAULT 0 NOT NULL,
    appreciation_pct_bps integer DEFAULT 0,
    status character varying(30) DEFAULT 'active'::character varying NOT NULL,
    payout_expected_at timestamp with time zone,
    purchased_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT investments_status_check CHECK (((status)::text = ANY ((ARRAY['active'::character varying, 'funded'::character varying, 'rented'::character varying, 'payout_pending'::character varying, 'in_process'::character varying, 'funding_in_progress'::character varying, 'exited'::character varying])::text[]))),
    CONSTRAINT investments_tokens_owned_check CHECK ((tokens_owned > 0))
);


ALTER TABLE public.investments OWNER TO martin;

--
-- Name: invoice_number_seq; Type: SEQUENCE; Schema: public; Owner: martin
--

CREATE SEQUENCE public.invoice_number_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.invoice_number_seq OWNER TO martin;

--
-- Name: invoices; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.invoices (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    invoice_number character varying(30) NOT NULL,
    order_id uuid NOT NULL,
    user_id uuid NOT NULL,
    company_entity character varying(255) DEFAULT 'POOOL GmbH'::character varying NOT NULL,
    subtotal_cents bigint NOT NULL,
    tax_cents bigint DEFAULT 0 NOT NULL,
    total_cents bigint NOT NULL,
    currency character varying(3) DEFAULT 'USD'::character varying NOT NULL,
    pdf_url character varying(512),
    status character varying(20) DEFAULT 'issued'::character varying NOT NULL,
    notes text,
    issued_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT invoices_status_check CHECK (((status)::text = ANY ((ARRAY['draft'::character varying, 'issued'::character varying, 'void'::character varying])::text[])))
);


ALTER TABLE public.invoices OWNER TO martin;

--
-- Name: kyc_documents; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.kyc_documents (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    document_type character varying(50) NOT NULL,
    gcs_path text NOT NULL,
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    rejection_reason text,
    kyc_record_id uuid,
    uploaded_at timestamp with time zone DEFAULT now() NOT NULL,
    reviewed_at timestamp with time zone,
    CONSTRAINT kyc_documents_document_type_check CHECK (((document_type)::text = ANY ((ARRAY['passport'::character varying, 'national_id'::character varying, 'driving_licence'::character varying, 'proof_of_address'::character varying, 'other'::character varying])::text[]))),
    CONSTRAINT kyc_documents_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'approved'::character varying, 'rejected'::character varying])::text[])))
);


ALTER TABLE public.kyc_documents OWNER TO martin;

--
-- Name: kyc_records; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.kyc_records (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    provider character varying(50) DEFAULT 'sumsub'::character varying NOT NULL,
    provider_ref_id character varying(255),
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    rejection_reason text,
    document_type character varying(50),
    pep_check_passed boolean,
    sanctions_check boolean,
    verified_at timestamp with time zone,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT kyc_records_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'in_review'::character varying, 'approved'::character varying, 'rejected'::character varying, 'expired'::character varying])::text[])))
);


ALTER TABLE public.kyc_records OWNER TO martin;

--
-- Name: notifications; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.notifications (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    title character varying(255) NOT NULL,
    message text,
    type character varying(30) NOT NULL,
    is_read boolean DEFAULT false NOT NULL,
    action_url character varying(512),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT notifications_type_check CHECK (((type)::text = ANY ((ARRAY['kyc'::character varying, 'investment'::character varying, 'payout'::character varying, 'system'::character varying, 'promo'::character varying])::text[])))
);


ALTER TABLE public.notifications OWNER TO martin;

--
-- Name: oauth_accounts; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.oauth_accounts (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    provider character varying(20) NOT NULL,
    provider_id character varying(255) NOT NULL,
    provider_email character varying(255),
    access_token text,
    refresh_token text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT oauth_accounts_provider_check CHECK (((provider)::text = ANY ((ARRAY['google'::character varying, 'facebook'::character varying, 'apple'::character varying])::text[])))
);


ALTER TABLE public.oauth_accounts OWNER TO martin;

--
-- Name: order_items; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.order_items (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    order_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    tokens_quantity integer NOT NULL,
    token_price_cents bigint NOT NULL,
    subtotal_cents bigint NOT NULL
);


ALTER TABLE public.order_items OWNER TO martin;

--
-- Name: orders; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.orders (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    order_number character varying(30) NOT NULL,
    total_cents bigint NOT NULL,
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    payment_method character varying(30),
    payment_ref_id character varying(255),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    currency character varying(3) DEFAULT 'USD'::character varying NOT NULL,
    payment_currency character varying(3),
    fx_rate numeric(18,8),
    fx_provider character varying(50),
    CONSTRAINT orders_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'processing'::character varying, 'completed'::character varying, 'failed'::character varying, 'cancelled'::character varying, 'refunded'::character varying])::text[])))
);


ALTER TABLE public.orders OWNER TO martin;

--
-- Name: password_reset_tokens; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.password_reset_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    token_hash character varying(255) NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    used_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.password_reset_tokens OWNER TO martin;

--
-- Name: payment_methods; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.payment_methods (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    method_type character varying(20) NOT NULL,
    processor_type character varying(50),
    processor_token character varying(255),
    customer_id character varying(255),
    brand character varying(50),
    last_four character varying(10),
    expiry_month integer,
    expiry_year integer,
    holder_name character varying(255),
    routing_number character varying(50),
    bank_country character varying(50),
    label character varying(255),
    is_default boolean DEFAULT false NOT NULL,
    status character varying(20) DEFAULT 'active'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.payment_methods OWNER TO martin;

--
-- Name: platform_settings; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.platform_settings (
    key character varying(100) NOT NULL,
    value text NOT NULL,
    value_type character varying(20) DEFAULT 'string'::character varying NOT NULL,
    description character varying(500),
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by uuid,
    CONSTRAINT platform_settings_value_type_check CHECK (((value_type)::text = ANY ((ARRAY['string'::character varying, 'number'::character varying, 'boolean'::character varying, 'json'::character varying])::text[])))
);


ALTER TABLE public.platform_settings OWNER TO martin;

--
-- Name: referral_clicks; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.referral_clicks (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    code character varying(32) NOT NULL,
    ip_address inet,
    user_agent text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    subid character varying(255)
);


ALTER TABLE public.referral_clicks OWNER TO martin;

--
-- Name: referral_codes; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.referral_codes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    code character varying(32) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.referral_codes OWNER TO martin;

--
-- Name: referral_tracking; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.referral_tracking (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    referrer_id uuid NOT NULL,
    referred_id uuid NOT NULL,
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    referrer_reward bigint DEFAULT 3000 NOT NULL,
    referred_reward bigint DEFAULT 3000 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    qualified_at timestamp with time zone,
    subid character varying(255)
);


ALTER TABLE public.referral_tracking OWNER TO martin;

--
-- Name: rewards_balances; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.rewards_balances (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    cashback bigint DEFAULT 0 NOT NULL,
    referrals bigint DEFAULT 0 NOT NULL,
    promotions bigint DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.rewards_balances OWNER TO martin;

--
-- Name: roles; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.roles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name character varying(30) NOT NULL,
    description character varying(255)
);


ALTER TABLE public.roles OWNER TO martin;

--
-- Name: support_ticket_attachments; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.support_ticket_attachments (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    reply_id uuid NOT NULL,
    file_url character varying(512) NOT NULL,
    file_type character varying(100),
    file_size_bytes bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.support_ticket_attachments OWNER TO martin;

--
-- Name: support_ticket_replies; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.support_ticket_replies (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    ticket_id uuid NOT NULL,
    author_id uuid NOT NULL,
    author_name character varying(200) NOT NULL,
    author_role character varying(20) DEFAULT 'user'::character varying NOT NULL,
    type character varying(30) DEFAULT 'reply'::character varying NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT support_ticket_replies_author_role_check CHECK (((author_role)::text = ANY ((ARRAY['customer'::character varying, 'user'::character varying, 'agent'::character varying, 'admin'::character varying])::text[]))),
    CONSTRAINT support_ticket_replies_type_check CHECK (((type)::text = ANY ((ARRAY['initial'::character varying, 'reply'::character varying, 'internal_note'::character varying])::text[])))
);


ALTER TABLE public.support_ticket_replies OWNER TO martin;

--
-- Name: support_tickets; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.support_tickets (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    subject character varying(255) NOT NULL,
    message text NOT NULL,
    status character varying(20) DEFAULT 'open'::character varying NOT NULL,
    priority character varying(10) DEFAULT 'normal'::character varying,
    assigned_to uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    category character varying(50),
    metadata jsonb,
    sla_breach_at timestamp with time zone,
    sla_alert_sent boolean DEFAULT false NOT NULL,
    csat_score smallint,
    csat_feedback text,
    CONSTRAINT support_tickets_csat_score_check CHECK (((csat_score >= 1) AND (csat_score <= 5))),
    CONSTRAINT support_tickets_priority_check CHECK (((priority)::text = ANY ((ARRAY['low'::character varying, 'normal'::character varying, 'high'::character varying, 'urgent'::character varying])::text[]))),
    CONSTRAINT support_tickets_status_check CHECK (((status)::text = ANY ((ARRAY['open'::character varying, 'in_progress'::character varying, 'waiting_on_customer'::character varying, 'resolved'::character varying, 'closed'::character varying])::text[])))
);


ALTER TABLE public.support_tickets OWNER TO martin;

--
-- Name: tiers; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.tiers (
    id integer NOT NULL,
    name character varying(32) NOT NULL,
    min_invest bigint DEFAULT 0 NOT NULL,
    max_invest bigint,
    cashback_pct numeric(5,2) DEFAULT 0 NOT NULL,
    badge_color character varying(7) DEFAULT '#D0D5DD'::character varying NOT NULL,
    sort_order integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    referral_bonus bigint DEFAULT 3000
);


ALTER TABLE public.tiers OWNER TO martin;

--
-- Name: tiers_id_seq; Type: SEQUENCE; Schema: public; Owner: martin
--

CREATE SEQUENCE public.tiers_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.tiers_id_seq OWNER TO martin;

--
-- Name: tiers_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: martin
--

ALTER SEQUENCE public.tiers_id_seq OWNED BY public.tiers.id;


--
-- Name: user_consents; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.user_consents (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    terms_version character varying(50) DEFAULT '1.0'::character varying NOT NULL,
    accepted_at timestamp with time zone DEFAULT now() NOT NULL,
    ip_address character varying(100),
    user_agent text
);


ALTER TABLE public.user_consents OWNER TO martin;

--
-- Name: user_profiles; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.user_profiles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    first_name character varying(100),
    last_name character varying(100),
    display_name character varying(200),
    date_of_birth date,
    nationality character varying(3),
    address_line_1 character varying(255),
    address_line_2 character varying(255),
    city character varying(100),
    state_province character varying(100),
    postal_code character varying(20),
    country character varying(3),
    phone_number character varying(30),
    tax_id character varying(50),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_profiles OWNER TO martin;

--
-- Name: user_roles; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.user_roles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    role_id uuid NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    granted_at timestamp with time zone DEFAULT now() NOT NULL,
    authorized_ips inet[],
    access_start_time time without time zone,
    access_end_time time without time zone
);


ALTER TABLE public.user_roles OWNER TO martin;

--
-- Name: user_sessions; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.user_sessions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    session_token character varying(255) NOT NULL,
    ip_address inet,
    user_agent text,
    remember_me boolean DEFAULT false NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    is_2fa_verified boolean DEFAULT false NOT NULL
);


ALTER TABLE public.user_sessions OWNER TO martin;

--
-- Name: user_settings; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.user_settings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    totp_secret character varying(255),
    totp_enabled boolean DEFAULT false NOT NULL,
    language character varying(5) DEFAULT 'en'::character varying,
    email_notifications boolean DEFAULT true,
    push_notifications boolean DEFAULT true,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    currency character varying(3) DEFAULT 'USD'::character varying NOT NULL,
    timezone character varying(64) DEFAULT 'UTC'::character varying NOT NULL
);


ALTER TABLE public.user_settings OWNER TO martin;

--
-- Name: COLUMN user_settings.currency; Type: COMMENT; Schema: public; Owner: martin
--

COMMENT ON COLUMN public.user_settings.currency IS 'ISO 4217 currency code (USD, EUR, GBP, SGD, IDR)';


--
-- Name: COLUMN user_settings.timezone; Type: COMMENT; Schema: public; Owner: martin
--

COMMENT ON COLUMN public.user_settings.timezone IS 'IANA timezone identifier (e.g. America/New_York)';


--
-- Name: user_tiers; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.user_tiers (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    tier_id integer NOT NULL,
    invested_12m bigint DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_tiers OWNER TO martin;

--
-- Name: users; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    email character varying(255) NOT NULL,
    password_hash character varying(255),
    email_verified boolean DEFAULT false NOT NULL,
    avatar_url character varying(1024),
    status character varying(20) DEFAULT 'active'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    reset_password boolean DEFAULT false NOT NULL,
    CONSTRAINT users_status_check CHECK (((status)::text = ANY ((ARRAY['active'::character varying, 'suspended'::character varying, 'deleted'::character varying])::text[])))
);


ALTER TABLE public.users OWNER TO martin;

--
-- Name: wallet_transactions; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.wallet_transactions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    wallet_id uuid NOT NULL,
    type character varying(30) NOT NULL,
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    amount_cents bigint NOT NULL,
    description text,
    external_ref_id character varying(255),
    related_order_id uuid,
    metadata jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    currency character varying(3) DEFAULT 'USD'::character varying NOT NULL,
    CONSTRAINT wallet_transactions_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'processing'::character varying, 'completed'::character varying, 'failed'::character varying, 'cancelled'::character varying])::text[]))),
    CONSTRAINT wallet_transactions_type_check CHECK (((type)::text = ANY ((ARRAY['deposit'::character varying, 'withdrawal'::character varying, 'purchase'::character varying, 'sale'::character varying, 'dividend'::character varying, 'reward'::character varying, 'refund'::character varying, 'fee'::character varying])::text[])))
);


ALTER TABLE public.wallet_transactions OWNER TO martin;

--
-- Name: wallets; Type: TABLE; Schema: public; Owner: martin
--

CREATE TABLE public.wallets (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    wallet_type character varying(20) NOT NULL,
    balance_cents bigint DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    currency character varying(3) DEFAULT 'USD'::character varying NOT NULL,
    CONSTRAINT wallets_balance_cents_check CHECK ((balance_cents >= 0)),
    CONSTRAINT wallets_wallet_type_check CHECK (((wallet_type)::text = ANY ((ARRAY['cash'::character varying, 'rewards'::character varying])::text[])))
);


ALTER TABLE public.wallets OWNER TO martin;

--
-- Name: audit_logs id; Type: DEFAULT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.audit_logs ALTER COLUMN id SET DEFAULT nextval('public.audit_logs_id_seq'::regclass);


--
-- Name: email_logs id; Type: DEFAULT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.email_logs ALTER COLUMN id SET DEFAULT nextval('public.email_logs_id_seq'::regclass);


--
-- Name: tiers id; Type: DEFAULT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.tiers ALTER COLUMN id SET DEFAULT nextval('public.tiers_id_seq'::regclass);


--
-- Data for Name: admin_approval_requests; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.admin_approval_requests (id, requester_id, approver_id, action_type, entity_type, entity_id, payload, status, rejection_reason, expires_at, created_at, updated_at) FROM stdin;
5f590798-645c-4df7-b36c-b63b29f3fb67	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	dividend.process	asset	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	{"asset_id": "6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771", "total_amount_cents": 100}	pending	\N	2026-03-09 23:52:03.323945+07	2026-03-08 23:52:03.323945+07	2026-03-08 23:52:03.323945+07
\.


--
-- Data for Name: admin_invitations; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.admin_invitations (id, email, role_id, invited_by, token_hash, status, expires_at, created_at, accepted_at) FROM stdin;
\.


--
-- Data for Name: admin_permissions; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.admin_permissions (id, role_id, permission) FROM stdin;
c60d778b-df23-4b2c-aaed-44bddd8e2aa4	348274de-230c-4ed5-8c5f-6ec0952d46f0	all
781a60d6-76a9-4959-b280-d6e83a49e2a1	3368e3a7-10ac-4e20-9c3b-015956bddca0	all
0a66f655-28c0-4acc-978a-6ee0968d3d80	71b01f28-a9b5-4918-9f99-942e58c2b5e3	kyc.read
95ebeb70-a583-484e-9ff2-1501b5abb3b2	71b01f28-a9b5-4918-9f99-942e58c2b5e3	kyc.write
fbfe58de-ff6d-4cfc-a8d9-b433619fa39b	71b01f28-a9b5-4918-9f99-942e58c2b5e3	users.read
bdb9c5e8-a9b2-4eb8-986d-f4d63ad618fb	271b2cb4-0d3f-40a6-a402-9a4c61d1c158	support.read
d9041103-bc07-4f21-9d0d-c72028b82ce2	271b2cb4-0d3f-40a6-a402-9a4c61d1c158	support.write
2673b583-3a1e-4a15-b582-7312a9e3ebea	271b2cb4-0d3f-40a6-a402-9a4c61d1c158	users.read
9c042f40-5a75-4ad2-809c-19a58d0f417f	99ccc83b-f91f-4409-8662-8e493249f4a7	treasury.read
85ebb434-2c36-4666-9f24-2ee63775c353	99ccc83b-f91f-4409-8662-8e493249f4a7	treasury.write
38d0355b-6cce-4cb9-89a2-da138c1522f5	99ccc83b-f91f-4409-8662-8e493249f4a7	deposits.read
2ede76c6-20c8-4aa7-84a7-f4a69873dc7d	99ccc83b-f91f-4409-8662-8e493249f4a7	deposits.write
ee248552-546e-44f1-b3d2-a746f96963f8	99ccc83b-f91f-4409-8662-8e493249f4a7	orders.read
e9023a52-5bb3-464f-ab5b-fb1611f00438	3368e3a7-10ac-4e20-9c3b-015956bddca0	admins.manage
e4586df4-44bd-4cd0-8aa1-c4a5d2faca7e	3368e3a7-10ac-4e20-9c3b-015956bddca0	roles.edit
ea554ba7-60d1-47b8-9fb4-76f6466989f2	3368e3a7-10ac-4e20-9c3b-015956bddca0	pii.view
f01e792b-898d-4b56-9c2c-df273d8f1d37	3368e3a7-10ac-4e20-9c3b-015956bddca0	financials.payout.draft
8a80b8d4-a8ed-45db-a7e3-6da89eb14cc6	3368e3a7-10ac-4e20-9c3b-015956bddca0	financials.payout.approve
6768d4e9-0a6a-45bf-baab-f8fcac2336a5	71b01f28-a9b5-4918-9f99-942e58c2b5e3	pii.view
f826375a-b9f0-492d-b587-1041011d108a	71b01f28-a9b5-4918-9f99-942e58c2b5e3	kyc.override
564c568c-8fa9-4d19-afcd-5dca29a0b244	99ccc83b-f91f-4409-8662-8e493249f4a7	financials.payout.draft
\.


--
-- Data for Name: asset_documents; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.asset_documents (id, asset_id, document_type, title, file_url, file_size_bytes, created_at) FROM stdin;
1f5b4137-11ad-4859-9aec-84c60e608063	0a64f742-eb99-41f7-bd0a-9dd941b11011	expose	Investment Expose – Clifftop Villa	/docs/expose-clifftop-villa.pdf	2456780	2026-03-08 20:12:49.484753+07
238b3579-bdb9-41e1-b5d4-d5cdb3219cd7	0a64f742-eb99-41f7-bd0a-9dd941b11011	appraisal	Independent Appraisal Report	/docs/appraisal-clifftop-villa.pdf	1234567	2026-03-08 20:12:49.484753+07
b1a4627a-56fb-47fd-838f-b4a3855fa579	0a64f742-eb99-41f7-bd0a-9dd941b11011	proof_of_title	Certificate of Leasehold	/docs/title-clifftop-villa.pdf	345678	2026-03-08 20:12:49.484753+07
8fc58d7e-dd74-4200-b776-d6b9f3537399	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	expose	Investment Expose – Surf Villa Canggu	/docs/expose-surf-villa.pdf	1987654	2026-03-08 20:12:49.484753+07
3eaf8925-4b3f-4b00-bd3d-26755602bbcd	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	financial	Financial Projections 5-Year	/docs/financials-surf-villa.pdf	567890	2026-03-08 20:12:49.484753+07
8fc3e3b1-bd68-4e6c-8dfe-7c384fb2e429	ebdc983b-5792-43a0-b764-1c0517701e2b	expose	Renovation Project Plan	/docs/expose-renovation-flip.pdf	3456789	2026-03-08 20:12:49.484753+07
bd96dd1b-529d-4c87-88e3-83507ccab459	ebdc983b-5792-43a0-b764-1c0517701e2b	floor_plan	Proposed Floor Plans	/docs/floorplan-renovation.pdf	987654	2026-03-08 20:12:49.484753+07
\.


--
-- Data for Name: asset_financials; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.asset_financials (id, asset_id, period_month, period_year, rental_income_cents, appreciation_cents, occupancy_rate_bps, expenses_cents, net_income_cents, created_at) FROM stdin;
2a63ab4b-4f2c-4d92-82f7-84ae077a8228	0a64f742-eb99-41f7-bd0a-9dd941b11011	1	2026	1200000	0	9200	285000	915000	2026-03-08 20:12:49.484753+07
2a0c4627-03a1-4a42-bce3-15512d86b87a	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	2026	1150000	0	8800	275000	875000	2026-03-08 20:12:49.484753+07
a4dae5ee-331e-4db4-81c9-774fc0e97d69	0a64f742-eb99-41f7-bd0a-9dd941b11011	3	2026	1350000	0	9500	295000	1055000	2026-03-08 20:12:49.484753+07
d278b123-4847-41f1-b694-4ecdf8fd805e	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	1	2026	980000	0	8700	232000	748000	2026-03-08 20:12:49.484753+07
a68aa296-018f-45f1-b75f-15837f36492f	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	2	2026	1020000	0	9000	240000	780000	2026-03-08 20:12:49.484753+07
4aa7e265-87d4-49d9-9a57-fa17b9ea0fb5	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	3	2026	950000	0	8500	225000	725000	2026-03-08 20:12:49.484753+07
\.


--
-- Data for Name: asset_images; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.asset_images (id, asset_id, image_url, alt_text, sort_order, is_cover, created_at) FROM stdin;
b16f4b78-7465-45c0-ac0e-69e0cb00151e	0a64f742-eb99-41f7-bd0a-9dd941b11011	/images/villa1.webp	Luxury Clifftop Villa exterior	0	t	2026-03-08 20:12:49.484753+07
c7af78db-c92b-4a75-af61-76e3f7b29bb3	0a64f742-eb99-41f7-bd0a-9dd941b11011	/images/villa1_2.webp	Infinity pool with ocean view	1	f	2026-03-08 20:12:49.484753+07
a35dc3ac-50d8-4a7d-a785-a9fc44649ace	0a64f742-eb99-41f7-bd0a-9dd941b11011	/images/villa1_3.webp	Modern interior living area	2	f	2026-03-08 20:12:49.484753+07
19a3617d-f74a-443f-b0d7-af0745f4d7ac	0a64f742-eb99-41f7-bd0a-9dd941b11011	/images/villa1_4.webp	Ocean view terrace	3	f	2026-03-08 20:12:49.484753+07
83592069-d268-477d-b099-8e0150f264f1	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	/images/villa2_1.webp	Modern Surf Villa exterior	0	t	2026-03-08 20:12:49.484753+07
fe03db7f-4da3-46b1-8d6f-f52fe3c869b5	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	/images/villa2_2.webp	Tropical garden and pool	1	f	2026-03-08 20:12:49.484753+07
cf8b7c3b-c946-4ed7-9618-e8dab9081562	fef0838f-01cd-4993-8c86-0aa7cd6e1cbf	/images/villa3_1.webp	Boutique Resort entrance	0	t	2026-03-08 20:12:49.484753+07
946504a5-5cd6-4a9e-ba1c-f6932a69debc	fef0838f-01cd-4993-8c86-0aa7cd6e1cbf	/images/villa3_2.webp	Resort common area	1	f	2026-03-08 20:12:49.484753+07
4f9f91ef-acce-4293-893b-526d2158c0c6	0303245f-a4b0-4723-87f7-3cfcd64910d5	/images/villa4_1.webp	Vacation Rental Villa	0	t	2026-03-08 20:12:49.484753+07
7fc58a58-d595-46da-aeb8-34e4a1c6e847	0303245f-a4b0-4723-87f7-3cfcd64910d5	/images/villa4_2.webp	Panoramic temple view	1	f	2026-03-08 20:12:49.484753+07
e1bdd803-54ce-4bdb-ba51-1c7f36be4aef	ebdc983b-5792-43a0-b764-1c0517701e2b	/images/villa5.webp	Renovation flip project	0	t	2026-03-08 20:12:49.484753+07
931f4cfb-3f78-4a0f-b9ef-af3a8208a391	e8719314-1fac-4e2a-ad2d-e86a725fec96	/images/villa6.webp	New development site	0	t	2026-03-08 20:12:49.484753+07
0fb37aba-379e-4b71-9e74-fe2d1fafa681	89295282-6302-42d3-9961-a168b4578a40	/images/villa3_1.webp	Funded pool villa	0	t	2026-03-08 20:12:49.484753+07
fdbe1bdf-f6e7-4659-9de1-b2f6ff88732f	89295282-6302-42d3-9961-a168b4578a40	/images/villa1_2.webp	Pool area	1	f	2026-03-08 20:12:49.484753+07
4f467769-af8c-4f3b-9553-dbfb60a22168	cb932002-0cdd-46e4-bb35-209e658060c9	/images/villa2_1.webp	Beachfront retreat	0	t	2026-03-08 20:12:49.484753+07
f0991255-c4bc-4ba5-bdb1-453728654274	fad407a3-b106-4903-bd4e-0772fc94c78e	/static/images/commodities/rice/eduardo-prim-3u51-uLQICc-unsplash.webp	Bali rice terraces	0	t	2026-03-08 20:12:49.484753+07
10de4123-2874-4613-b475-0ca125a07a34	fad407a3-b106-4903-bd4e-0772fc94c78e	/static/images/commodities/rice/zhao-yangjun-dDAzpSUAbgI-unsplash.webp	Rice harvest	1	f	2026-03-08 20:12:49.484753+07
392acc53-75dd-4f1e-8d26-4c6ef6359cac	0081d064-996e-46a2-b19c-92a9d7389767	/static/images/commodities/rice/hoach-le-dinh-PeRt3uMmjYM-unsplash.webp	Cacao beans	0	t	2026-03-08 20:12:49.484753+07
bbd4e2f8-5670-4cbe-8e26-0941416d702f	754e5be6-22f6-44fb-8cd7-f3aa90d86d25	/static/images/commodities/rice/winston-chen-kXoEdaZ3SFw-unsplash.webp	Coffee beans	0	t	2026-03-08 20:12:49.484753+07
a8be29ee-13ea-4361-97b4-1702a351db10	a1111111-1111-1111-1111-111111111111	/images/villa1.jpg	Sunset Heights Villa exterior	0	t	2026-03-09 14:54:18.329653+07
759e9320-7151-45ad-9889-34c960a0eb9f	a1111111-1111-1111-1111-111111111111	/images/villa1_2.jpg	Sunset Heights Villa pool	1	f	2026-03-09 14:54:18.329653+07
3a924b9f-973d-437d-9f01-de3e4fb5d754	a2222222-2222-2222-2222-222222222222	/images/villa3_1.jpg	Central Plaza Commerce exterior	0	t	2026-03-09 14:54:18.329653+07
243c8e8d-0d3f-4e3c-920a-05dba56e9e18	a2222222-2222-2222-2222-222222222222	/images/villa3_2.jpg	Central Plaza Commerce interior	1	f	2026-03-09 14:54:18.329653+07
4ca8778c-369e-4a12-b4bc-2618b000e130	a3333333-3333-3333-3333-333333333333	/static/images/commodities/rice/eduardo-prim-3u51-uLQICc-unsplash.jpg	Green Field Agriculture – lush farmland	0	t	2026-03-09 14:54:18.329653+07
3420cb1b-3e34-4b43-94d3-4318a907c445	a4444444-4444-4444-4444-444444444444	/images/villa4_1.jpg	Uluwatu Luxury Retreat exterior	0	t	2026-03-09 14:54:18.329653+07
b80e35da-9cb6-4541-8cd9-9ccb25bd3b6c	a4444444-4444-4444-4444-444444444444	/images/villa1_2.jpg	Uluwatu Luxury Retreat pool	1	f	2026-03-09 14:54:18.329653+07
\.


--
-- Data for Name: asset_milestones; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.asset_milestones (id, asset_id, title, description, milestone_date, month_index, is_completed, created_at) FROM stdin;
9e6ed35b-e1cb-4a8d-8310-d92391cfc0d5	0a64f742-eb99-41f7-bd0a-9dd941b11011	Funding Opened	Funding campaign launched on POOOL Platform	2026-01-07 20:12:49.484753+07	0	t	2026-03-08 20:12:49.484753+07
281ee6c6-39f3-46c1-87c6-593da193f0cc	0a64f742-eb99-41f7-bd0a-9dd941b11011	50% Funded	Reached 50% of funding target	2026-02-06 20:12:49.484753+07	1	t	2026-03-08 20:12:49.484753+07
aed8c027-6dda-40ba-9a27-bcd757195692	0a64f742-eb99-41f7-bd0a-9dd941b11011	89% Funded	Nearing full funding!	2026-03-03 20:12:49.484753+07	2	t	2026-03-08 20:12:49.484753+07
d711a551-dc15-4ed6-a252-3c79f4b365b5	0a64f742-eb99-41f7-bd0a-9dd941b11011	Fully Funded	Target: 100% funded and property acquired	2026-04-07 20:12:49.484753+07	3	f	2026-03-08 20:12:49.484753+07
04df8563-60d4-4cff-a5c1-69189399cebb	0a64f742-eb99-41f7-bd0a-9dd941b11011	First Rental Income	Expected first rental payout to investors	2026-06-06 20:12:49.484753+07	4	f	2026-03-08 20:12:49.484753+07
4035511d-f62b-43b6-a715-21554589623b	ebdc983b-5792-43a0-b764-1c0517701e2b	Funding Opened	Renovation project funding launched	2026-02-26 20:12:49.484753+07	0	t	2026-03-08 20:12:49.484753+07
835d4a3d-05a4-40e5-b5fc-f156d7e3d298	ebdc983b-5792-43a0-b764-1c0517701e2b	Property Acquired	Purchase of villa completed	\N	1	f	2026-03-08 20:12:49.484753+07
9c251045-dd0c-4c02-89da-4f131fd997fd	ebdc983b-5792-43a0-b764-1c0517701e2b	Renovation Start	Construction work begins	\N	2	f	2026-03-08 20:12:49.484753+07
6a6f7bac-d407-4724-8327-0d65baa4be88	ebdc983b-5792-43a0-b764-1c0517701e2b	Renovation Complete	All work finished, final inspection	\N	5	f	2026-03-08 20:12:49.484753+07
7368d036-f06a-4329-bc51-3c415c16c250	ebdc983b-5792-43a0-b764-1c0517701e2b	Exit / Sale	Property listed for sale	\N	8	f	2026-03-08 20:12:49.484753+07
\.


--
-- Data for Name: assets; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.assets (id, developer_user_id, title, slug, short_description, description, asset_type, property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm, bedrooms, bathrooms, construction_status, year_built, location_city, location_country, location_address, location_lat, location_lng, location_description, google_maps_url, video_url, total_value_cents, token_price_cents, tokens_total, tokens_available, annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps, operator_name, term_months, fixed_roi_bps, revenue_min_cents, revenue_max_cents, expenses_cents, net_profit_min_cents, net_profit_max_cents, investor_payout_cents, operator_split_pct, poool_split_pct, funding_status, featured, published, funding_start_at, funding_end_at, created_at, updated_at) FROM stdin;
0a64f742-eb99-41f7-bd0a-9dd941b11011	c87443dc-b777-47b2-a1f0-e345c92e1b47	Luxury Clifftop Villa with Ocean Views in Uluwatu	luxury-clifftop-villa-uluwatu	Stunning clifftop villa with infinity pool and panoramic Indian Ocean views.	This exceptional 4-bedroom villa sits atop the dramatic limestone cliffs of Uluwatu, offering 180-degree views of the Indian Ocean. Features include an infinity edge pool, open-air living pavilion, professional kitchen, and private access to a secluded beach. The property generates consistent rental income through premium short-term vacation bookings.	real_estate	villa	Uluwatu	leasehold	25	800.00	450.00	4	5	ready	2022	Bali	ID	Jl. Pantai Suluban, Uluwatu, Pecatu, Bali 80364	-8.8113000	115.0887000	Perched on the clifftops of Uluwatu with stunning ocean views, 15 minutes from Padang Padang Beach.	\N	\N	133400000	13340	10000	8822	1050	800	8900	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funding_in_progress	t	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 21:45:14.723443+07
754e5be6-22f6-44fb-8cd7-f3aa90d86d25	c87443dc-b777-47b2-a1f0-e345c92e1b47	Specialty Coffee – Kintamani Highlands	specialty-coffee-kintamani-2026	Kintamani single-origin coffee production – fully funded.	A fully funded investment in specialty-grade Arabica coffee grown on the volcanic slopes of Mount Batur. Beans are processed using the wet-hull method and sold to specialty roasters in Australia and Japan.	commodity	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	Bali	ID	\N	\N	\N	\N	\N	\N	3000000	3000	1000	0	\N	\N	\N	Kintamani Coffee Farmers	9	1100	\N	\N	\N	\N	\N	\N	\N	\N	funded	f	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 16:00:00.96097+07
cb932002-0cdd-46e4-bb35-209e658060c9	\N	Beachfront Retreat – Successfully Exited	beachfront-retreat-sanur-exited	This investment has successfully exited with a 42% total return to investors.	A beautiful 2-bedroom beachfront retreat in Sanur that was acquired, renovated, and successfully sold after 18 months. All investors received their principal plus 42% total return. This property demonstrates the profit potential of Bali real estate investments.	real_estate	villa	Sanur	freehold	\N	350.00	180.00	2	2	ready	2019	Bali	ID	Jl. Pantai Karang, Sanur, Bali 80228	-8.6903000	115.2619000	\N	\N	\N	65000000	6500	10000	0	0	4200	0	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	exited	t	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 13:50:33.763767+07
a2222222-2222-2222-2222-222222222222	0ee84d65-6576-4d63-b767-0359b06b8ec5	Central Plaza Commerce	central-plaza-commerce	\N	\N	commercial_property	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	Denpasar	ID	\N	\N	\N	\N	\N	\N	500000000	100000	5000	5000	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	upcoming	f	t	\N	\N	2026-03-03 22:04:38.718487+07	2026-03-09 01:03:09.853265+07
a3333333-3333-3333-3333-333333333333	0ee84d65-6576-4d63-b767-0359b06b8ec5	Green Field Agriculture	green-field-agriculture	\N	\N	commodity	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	Tabanan	ID	\N	\N	\N	\N	\N	\N	75000000	25000	3000	3000	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	upcoming	f	t	\N	\N	2026-02-26 22:04:38.718487+07	2026-03-09 01:03:09.853265+07
e8719314-1fac-4e2a-ad2d-e86a725fec96	\N	New Development Project – 4 Villa Complex	new-development-seminyak	Ground-up development of 4 luxury villas in the heart of Seminyak.	An exciting development project to build 4 luxury 2-bedroom villas on a premium 1200sqm plot in central Seminyak. Close to beach, restaurants, and nightlife. Units will be sold individually or operated as vacation rentals upon completion. Professional developer with proven track record.	real_estate	villa	Seminyak	leasehold	25	1200.00	\N	8	8	construction	\N	Bali	ID	Jl. Kayu Aya, Seminyak, Bali 80361	-8.6815000	115.1580000	\N	\N	\N	180000000	18000	10000	6989	850	1200	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funding_open	t	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 21:46:48.206756+07
6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	\N	Modern Surf Villa near Echo Beach in Canggu	modern-surf-villa-canggu	Contemporary villa steps from Echo Beach, Canggu's most popular surf break.	Modern 4-bedroom freehold villa located just 200m from Echo Beach in the heart of Canggu. Features open-concept living, a 12m pool, rooftop terrace with sunset views, and a dedicated surf board storage. Positioned in Bali's fastest-growing tourist area with year-round high occupancy rates.	real_estate	villa	Canggu	freehold	\N	600.00	380.00	4	4	ready	2023	Bali	ID	Jl. Nelayan, Echo Beach, Canggu, Bali 80361	-8.6509000	115.1300000	\N	\N	\N	115000000	11500	10000	2400	1200	900	8500	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funding_in_progress	t	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 01:03:09.853265+07
fef0838f-01cd-4993-8c86-0aa7cd6e1cbf	\N	Boutique Resort with 6 Villas in Central Ubud	boutique-resort-ubud	Luxury boutique resort with 6 private villas surrounded by rice terraces.	A fully operational boutique resort featuring 6 individually designed villas, each with private pool, set amongst lush tropical gardens and rice paddies. Central Ubud location provides easy access to cultural attractions, restaurants, and yoga studios. Strong year-round bookings with premium nightly rates.	real_estate	commercial	Ubud	leasehold	30	2500.00	1200.00	12	14	ready	2020	Bali	ID	Jl. Kajeng, Ubud, Gianyar, Bali 80571	-8.5069000	115.2625000	\N	\N	\N	285000000	28500	10000	3600	1400	600	7800	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funding_in_progress	f	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 01:03:09.853265+07
0303245f-a4b0-4723-87f7-3cfcd64910d5	\N	Vacation Rental Villa with Temple Views	vacation-rental-villa-uluwatu	Charming 3-bedroom villa with views of the iconic Uluwatu Temple.	A beautifully designed 3-bedroom villa offering stunning views of Uluwatu Temple and the ocean. Features traditional Balinese architecture with modern amenities, a private pool, outdoor dining area, and tropical garden. Generates excellent short-term rental income with high occupancy during peak season.	real_estate	villa	Uluwatu	leasehold	20	500.00	280.00	3	3	ready	2021	Bali	ID	Jl. Pura Uluwatu, Pecatu, Kuta, Bali 80361	-8.8295000	115.0849000	\N	\N	\N	78500000	7850	10000	800	1300	700	9200	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funding_in_progress	f	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 01:03:09.853265+07
ebdc983b-5792-43a0-b764-1c0517701e2b	\N	Renovation Flip Project – Canggu Villa	renovation-flip-canggu	Value-add renovation opportunity to flip a dated villa in prime Canggu location.	Short-term investment opportunity: Acquire and renovate a dated 3-bedroom villa on a prime 400sqm plot in central Canggu. Plans include full interior renovation, new pool, and landscaping. Target flip within 12-18 months for significant capital appreciation. Professional project management team in place.	real_estate	villa	Canggu	leasehold	20	400.00	220.00	3	3	renovation	\N	Bali	ID	Jl. Batu Bolong, Canggu, Bali 80361	-8.6483000	115.1345000	\N	\N	\N	45000000	4500	10000	5500	0	2500	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funding_open	f	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 01:03:09.853265+07
89295282-6302-42d3-9961-a168b4578a40	\N	Luxury Pool Villa – Fully Funded	luxury-pool-villa-canggu-funded	This villa has been fully funded and is now generating rental income.	A stunning 3-bedroom pool villa in Berawa, Canggu. Currently fully occupied and generating strong monthly returns for investors. Managed by a professional villa management company with transparent monthly reporting.	real_estate	villa	Canggu	leasehold	25	500.00	300.00	3	4	ready	2023	Bali	ID	Jl. Pantai Berawa, Canggu, Bali 80361	-8.6538000	115.1400000	\N	\N	\N	95000000	9500	10000	0	1100	700	9000	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funded	f	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 01:03:09.853265+07
fad407a3-b106-4903-bd4e-0772fc94c78e	\N	Premium Bali Rice – Harvest Cycle Q2 2026	premium-bali-rice-q2-2026	Invest in organic Bali rice production with fixed 12% ROI over 6 months.	Partner with established rice farmers in Tabanan to fund the next harvest cycle of premium organic Bali rice. The rice is sold to luxury hotels and exported to international markets. Fixed ROI with profit sharing above target.	commodity	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	Bali	ID	\N	\N	\N	\N	\N	\N	5000000	5000	1000	350	\N	\N	\N	PT Bali Rice Co.	6	1200	15000000	22000000	8000000	7000000	14000000	4000000	60	10	funding_in_progress	t	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 01:03:09.853265+07
a4444444-4444-4444-4444-444444444444	0ee84d65-6576-4d63-b767-0359b06b8ec5	Uluwatu Luxury Retreat	uluwatu-luxury-retreat	\N	\N	real_estate	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	Uluwatu	ID	\N	\N	\N	\N	\N	\N	250000000	100000	2500	0	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funded	f	t	\N	\N	2026-02-21 22:04:38.718487+07	2026-03-09 01:03:09.853265+07
a1111111-1111-1111-1111-111111111111	0ee84d65-6576-4d63-b767-0359b06b8ec5	Sunset Heights Villa	sunset-heights-villa	\N	\N	real_estate	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	Jimbaran	ID	\N	\N	\N	\N	\N	\N	120000000	50000	2400	2400	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	funding_open	f	t	\N	\N	2026-03-06 22:04:38.718487+07	2026-03-09 01:03:09.853265+07
0081d064-996e-46a2-b19c-92a9d7389767	\N	Organic Cacao – Single Origin Bali	organic-cacao-bali-2026	Fund single-origin cacao production with 15% projected annual return.	Support Bali's growing artisan chocolate industry by investing in organic cacao bean production. Beans are fermented and dried on-site, then sold to premium chocolate makers worldwide. Strong commodity demand with increasing prices.	commodity	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	Bali	ID	\N	\N	\N	\N	\N	\N	8000000	8000	1000	8	\N	\N	\N	Bali Cacao Collective	12	1500	20000000	30000000	10000000	10000000	20000000	6000000	55	10	funding_open	f	t	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 20:47:13.568846+07
\.


--
-- Data for Name: audit_logs; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.audit_logs (id, actor_user_id, action, entity_type, entity_id, previous_state, new_state, ip_address, user_agent, metadata, created_at) FROM stdin;
1	0ee84d65-6576-4d63-b767-0359b06b8ec5	investment_created	investment	\N	\N	\N	\N	\N	{"asset": "Luxury Clifftop Villa", "tokens": 50, "amount_usd": 6670}	2026-03-08 20:12:49.484753+07
2	0ee84d65-6576-4d63-b767-0359b06b8ec5	investment_created	investment	\N	\N	\N	\N	\N	{"asset": "Modern Surf Villa", "tokens": 30, "amount_usd": 3450}	2026-03-08 20:12:49.484753+07
3	0ee84d65-6576-4d63-b767-0359b06b8ec5	investment_created	investment	\N	\N	\N	\N	\N	{"asset": "Luxury Pool Villa", "tokens": 100, "amount_usd": 9500}	2026-03-08 20:12:49.484753+07
4	0ee84d65-6576-4d63-b767-0359b06b8ec5	cart_add	cart	\N	\N	\N	\N	\N	{"asset": "Modern Surf Villa", "tokens": 5}	2026-03-08 20:12:49.484753+07
5	0ee84d65-6576-4d63-b767-0359b06b8ec5	profile_updated	user_profile	\N	\N	\N	\N	\N	{"fields": ["first_name", "last_name", "phone_number", "country"]}	2026-03-08 20:12:49.484753+07
6	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	Mozilla/5.0 (Linux; Android 8.0.0; SM-G965U Build/R16NW) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Mobile Safari/537.36	\N	2026-03-08 20:18:17.978312+07
7	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-08 20:29:44.116296+07
8	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-08 20:36:29.308223+07
9	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	user.registered	user	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	\N	\N	\N	python-requests/2.32.5	\N	2026-03-08 22:12:00.177829+07
10	f91c696d-f3e2-4525-80e1-01a29d700062	user.registered	user	f91c696d-f3e2-4525-80e1-01a29d700062	\N	\N	\N	python-requests/2.32.5	\N	2026-03-08 22:12:20.980501+07
11	db7174f5-c055-470c-ad50-b969ccfea5a4	user.registered	user	db7174f5-c055-470c-ad50-b969ccfea5a4	\N	\N	\N	python-requests/2.32.5	\N	2026-03-08 22:12:45.791477+07
12	db7174f5-c055-470c-ad50-b969ccfea5a4	admin.roles_update	users	db7174f5-c055-470c-ad50-b969ccfea5a4	\N	{"new_roles": ["investor", "developer", "admin"]}	\N	\N	\N	2026-03-08 22:17:12.205554+07
13	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin.user_status_update	user	db7174f5-c055-470c-ad50-b969ccfea5a4	\N	{"status": "suspended"}	\N	\N	\N	2026-03-08 22:30:23.510289+07
14	c87443dc-b777-47b2-a1f0-e345c92e1b47	developer_project.review_started	developer_projects	b1111111-1111-1111-1111-111111111111	{"status": "submitted"}	{"status": "in_review"}	\N	\N	\N	2026-03-08 23:38:16.591527+07
15	c87443dc-b777-47b2-a1f0-e345c92e1b47	approval_request.created	admin_approval_requests	5f590798-645c-4df7-b36c-b63b29f3fb67	\N	{"entity_id": "6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771", "action_type": "dividend.process"}	\N	\N	\N	2026-03-08 23:52:03.332222+07
16	c87443dc-b777-47b2-a1f0-e345c92e1b47	developer_project.approved	developer_projects	b1111111-1111-1111-1111-111111111111	{"status": "in_review"}	{"status": "approved", "funding_status": "funding_open", "asset_published": true}	\N	\N	\N	2026-03-08 23:54:26.161557+07
17	c87443dc-b777-47b2-a1f0-e345c92e1b47	developer_project.approved	developer_projects	b1111111-1111-1111-1111-111111111111	{"status": "approved"}	{"status": "approved", "funding_status": "funding_open", "asset_published": true}	\N	\N	\N	2026-03-08 23:54:29.406986+07
18	c87443dc-b777-47b2-a1f0-e345c92e1b47	developer_project.approved	developer_projects	b1111111-1111-1111-1111-111111111111	{"status": "approved"}	{"status": "approved", "funding_status": "funding_open", "asset_published": true}	\N	\N	\N	2026-03-08 23:55:19.639252+07
19	c87443dc-b777-47b2-a1f0-e345c92e1b47	developer_project.rejected	developer_projects	b1111111-1111-1111-1111-111111111111	{"status": "approved"}	{"status": "rejected", "rejection_reason": "deete"}	\N	\N	\N	2026-03-08 23:55:29.998395+07
20	dee2990f-8070-4d34-ab17-e7fb2c041b8b	user.registered	user	dee2990f-8070-4d34-ab17-e7fb2c041b8b	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:08:00.613669+07
21	dee2990f-8070-4d34-ab17-e7fb2c041b8b	user.login	user	dee2990f-8070-4d34-ab17-e7fb2c041b8b	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:08:00.856209+07
22	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:29:12.578131+07
23	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:29:47.924994+07
24	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:29:53.949561+07
25	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:30:00.837535+07
26	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:31:19.84726+07
27	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:41:03.210937+07
28	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:43:29.350376+07
29	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:45:03.238562+07
30	0ee84d65-6576-4d63-b767-0359b06b8ec5	kyc.initiated	kyc_records	2e6f40ff-4ebf-46e8-91a8-fc1d103d567f	\N	{"status": "pending", "provider": "didit"}	\N	\N	\N	2026-03-09 01:45:22.623345+07
31	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:45:28.407433+07
32	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:45:34.751032+07
33	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:45:57.707137+07
34	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:46:02.315684+07
35	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:46:48.949851+07
36	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:47:25.951076+07
37	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:48:02.807367+07
38	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:48:28.15611+07
39	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:48:32.976075+07
40	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:49:02.900736+07
41	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:52:11.958563+07
42	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:52:46.621556+07
43	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:53:10.484068+07
44	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:53:50.963116+07
45	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:58:53.354941+07
46	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:59:02.732086+07
47	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 01:59:04.750533+07
48	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:02:46.107157+07
49	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:02:56.378571+07
50	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:03:10.832526+07
51	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	b8dd1ce9-6105-42c1-8444-ca122165b940	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:03:11.247641+07
52	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:04:19.626779+07
53	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	e60bcc93-16f1-4024-8d57-07a2b17db01a	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:04:20.122872+07
54	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:04:20.814797+07
55	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	fc3e1ee2-db83-4b29-9d81-64dfda73a2c8	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:04:21.228762+07
56	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:08:38.643563+07
57	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	7e081384-8ab3-4bbd-ad4b-f0ef0f54f491	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:08:39.123654+07
58	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:08:42.311317+07
59	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:09:31.867166+07
60	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:09:32.158262+07
61	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	68d0b021-ab3d-496a-8b75-08db3476718f	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:09:32.5834+07
62	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:09:34.796171+07
63	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	6fbc763b-0fb5-4323-8e48-60011a7982ea	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:09:35.221188+07
64	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:09:41.198126+07
65	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	0be5281d-777b-4dc5-81e5-b44de7a6425f	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:09:41.611415+07
66	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:09:59.644227+07
67	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	cd7f1b23-1d87-46b8-973a-66e2ac450bd0	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:10:00.070503+07
68	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:10:07.859115+07
69	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	ee7f4921-9f01-4d2a-a007-b152aae88916	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:10:08.253838+07
70	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:10:08.67531+07
71	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:10:32.323646+07
72	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	6331c341-1a8c-4251-b5cc-06e47eb31b61	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:10:32.743319+07
73	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:12:46.990419+07
74	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:12:49.063723+07
75	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	26f45b02-9083-408e-bd81-3338ac99be17	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:12:49.510624+07
76	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:12:52.868369+07
77	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:13:03.870422+07
78	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	2c8aea8c-eda1-45da-b8cf-9d99360d5824	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 02:13:04.294602+07
79	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 02:13:28.831823+07
80	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:12:25.792209+07
81	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:12:26.479727+07
82	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	5e571409-ab3d-4ba0-8d9b-1da600cd884a	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:12:26.956285+07
83	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:13:11.865242+07
84	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:13:24.198046+07
85	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:14:00.434119+07
86	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:14:06.379614+07
87	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 10:14:47.386242+07
88	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:14:53.409014+07
89	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:15:14.05+07
90	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:16:08.46141+07
91	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:16:18.147469+07
92	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	1744c09a-3103-4f9d-83d1-34385835d063	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:16:18.62515+07
93	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 10:18:52.530659+07
94	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:32:02.498569+07
95	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	77e6b57f-041b-4b17-82a8-55d96f99e9d7	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:32:03.169033+07
96	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:32:48.627336+07
97	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	ade11c77-0b08-490d-99f5-f2d2d89dd7f6	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:32:49.170805+07
98	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 10:33:19.585628+07
99	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 10:34:06.180371+07
100	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:35:51.664194+07
101	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	ef05cc6c-6335-4a17-9c31-b58a9a836ee7	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:35:52.20138+07
102	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:35:52.724869+07
103	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	1dc1529f-f1e7-4361-9229-bab782f445ce	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:35:53.207412+07
104	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:35:58.420325+07
105	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	3b9c83e5-0922-47f9-9e39-58a26046fb65	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:35:58.926192+07
106	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:37:37.326695+07
107	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	c4965f95-158f-4955-9dd1-3ec8e0963d8d	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:37:37.795949+07
108	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 10:37:53.084465+07
109	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	7edd981a-5f7c-4b2b-8c0c-56941b456962	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 10:37:53.548092+07
110	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 10:53:03.615759+07
111	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 11:06:23.77015+07
112	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	f3aa5275-ccb0-4fb7-a871-c955768385ee	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 11:06:24.306505+07
113	dee2990f-8070-4d34-ab17-e7fb2c041b8b	user.login	user	dee2990f-8070-4d34-ab17-e7fb2c041b8b	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 11:08:51.859156+07
114	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 11:10:26.306165+07
115	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 11:11:19.074279+07
116	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 11:11:36.472498+07
117	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 11:12:05.769407+07
118	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	curl/8.7.1	\N	2026-03-09 11:28:53.158524+07
119	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	113.23.36.123	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 11:50:04.377098+07
120	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	curl/8.7.1	\N	2026-03-09 11:58:01.167499+07
121	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	7f66ed20-7961-41ab-a432-8e03ffc7c97c	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 12:31:22.903322+07
122	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	e3fd0b07-b139-4f22-b903-2eeb4b2fbaa5	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 12:33:03.188747+07
123	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 13:03:35.077462+07
124	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 13:04:31.37122+07
125	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 13:20:15.591878+07
126	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	19ab2def-ef5f-4a6e-a9c4-6e25f6c028b2	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:03:23.103631+07
127	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	8b96ef77-caf8-4d9b-9ae8-37cc1fce554c	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:03:40.738182+07
128	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	88a38c6e-6433-4891-94be-ade8c8b27334	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:04:55.281419+07
129	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	19d63357-ee66-4958-b1a1-77c84d3b0b5e	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:08:05.963506+07
130	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	2a142c27-7e37-4f97-804b-52fab8a341ac	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:08:19.088165+07
131	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	b8bb9ea2-7306-48a2-b0f9-075c4c68db45	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:09:02.021988+07
132	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	b040a9a2-fb69-4357-853d-e1d56a965393	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:10:15.913392+07
133	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	2104199d-0fff-43cd-9def-27e998b79c25	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:12:31.729845+07
134	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	1fce8d81-c4c6-4639-a0a3-6150ca40638c	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:12:41.27867+07
135	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	87f34e8e-899c-488f-9d7a-93a2beecd2bc	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:13:04.234306+07
136	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	4df3ce86-2e20-4d12-9770-00354347ccb5	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:14:10.129231+07
137	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	b0359c77-5557-451a-adcb-0a1077cfeadb	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:14:27.96879+07
138	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.tier_override	users	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	{"new_tier": "Elite"}	\N	\N	\N	2026-03-09 14:14:28.920281+07
139	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	4c5a9699-e1bd-482d-904a-a6e21ec76959	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:15:04.096367+07
140	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	05752b40-3625-4edc-a991-1f257734cb7b	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:15:44.972231+07
141	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	curl/8.7.1	\N	2026-03-09 14:46:17.914229+07
142	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	curl/8.7.1	\N	2026-03-09 14:50:16.453069+07
143	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin.invite	admin_invitation	\N	\N	{"role": "support", "email": "newadmin@poool.finance"}	\N	\N	\N	2026-03-09 14:51:14.027469+07
144	0ee84d65-6576-4d63-b767-0359b06b8ec5	admin.roles_update	users	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	{"new_roles": ["investor", "admin"]}	\N	\N	\N	2026-03-09 14:51:26.335874+07
145	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin.user_status_update	users	b45f038b-3e75-41e7-806a-0db9ca5b9272	\N	{"status": "suspended"}	\N	\N	\N	2026-03-09 14:51:46.392889+07
146	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin.user_status_update	users	b45f038b-3e75-41e7-806a-0db9ca5b9272	\N	{"status": "active"}	\N	\N	\N	2026-03-09 14:51:46.525236+07
147	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	cf790796-a42d-494e-8239-de8a8709fcfc	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:55:13.223497+07
148	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	573260f7-fcbe-4193-8317-77c5f719f3f9	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 14:55:39.82646+07
149	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 14:56:31.533235+07
150	c87443dc-b777-47b2-a1f0-e345c92e1b47	user.login	user	c87443dc-b777-47b2-a1f0-e345c92e1b47	\N	\N	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	\N	2026-03-09 15:06:45.152512+07
151	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:46:49.699432+07
152	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:48:27.32091+07
153	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:49:07.190779+07
154	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:49:36.511338+07
155	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:50:16.835211+07
156	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:53:32.113395+07
157	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:53:56.664592+07
158	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 15:54:56.242383+07
159	0ee84d65-6576-4d63-b767-0359b06b8ec5	user.login	user	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:15:21.575777+07
160	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	33e2e6ab-9674-4948-ae20-cc0ab1670120	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 16:28:36.976888+07
161	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	6fa58552-b2db-4908-ace7-3cd55b600c40	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 16:29:17.644914+07
162	af18c46f-fd0c-4617-a959-5d7e2fb556b2	user.registered	user	af18c46f-fd0c-4617-a959-5d7e2fb556b2	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:40:20.52027+07
163	edc6dda5-e21d-420d-a088-003dbcb5a827	user.registered	user	edc6dda5-e21d-420d-a088-003dbcb5a827	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:40:38.704895+07
164	edc6dda5-e21d-420d-a088-003dbcb5a827	user.login	user	edc6dda5-e21d-420d-a088-003dbcb5a827	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:40:38.719747+07
165	3f16bd25-2aff-42df-b193-bbc3faffb12b	user.registered	user	3f16bd25-2aff-42df-b193-bbc3faffb12b	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:41:06.070974+07
166	3f16bd25-2aff-42df-b193-bbc3faffb12b	user.login	user	3f16bd25-2aff-42df-b193-bbc3faffb12b	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:41:06.085293+07
169	2e5509db-67a5-46dc-87bb-391c6727a782	user.registered	user	2e5509db-67a5-46dc-87bb-391c6727a782	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:42:05.858935+07
170	2e5509db-67a5-46dc-87bb-391c6727a782	user.login	user	2e5509db-67a5-46dc-87bb-391c6727a782	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:42:05.873247+07
171	893aef07-c9e2-403d-9d2b-27eabcfcd78f	user.registered	user	893aef07-c9e2-403d-9d2b-27eabcfcd78f	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:42:05.894218+07
172	893aef07-c9e2-403d-9d2b-27eabcfcd78f	user.login	user	893aef07-c9e2-403d-9d2b-27eabcfcd78f	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:42:05.908233+07
173	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	user.registered	user	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:49:01.736682+07
174	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	user.login	user	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:49:01.753666+07
175	230101db-e689-47ad-9307-076f4f0e55f1	user.registered	user	230101db-e689-47ad-9307-076f4f0e55f1	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:49:01.773826+07
176	230101db-e689-47ad-9307-076f4f0e55f1	user.login	user	230101db-e689-47ad-9307-076f4f0e55f1	\N	\N	\N	python-requests/2.32.5	\N	2026-03-09 16:49:01.787942+07
185	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	3d19040e-c2c0-4fdb-908d-29e3d0e4ac23	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 18:15:17.378169+07
188	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	e992d372-fec0-4041-a495-6d7b732415e3	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 19:28:31.785911+07
189	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	264fdd24-94e6-4261-9e53-b22d57d78ea6	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 19:30:02.542928+07
190	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	0ea0427a-84c4-4cda-acd6-51dc3321b1cd	\N	\N	\N	\N	{"payment_method": "bank"}	2026-03-09 19:34:24.036683+07
191	c87443dc-b777-47b2-a1f0-e345c92e1b47	checkout.completed	order	389b6003-0b0b-4f2d-af19-110099b874e4	\N	\N	\N	\N	{"payment_method": "bank"}	2026-03-09 20:47:13.568846+07
192	0ee84d65-6576-4d63-b767-0359b06b8ec5	checkout.completed	order	144ba81d-a451-4190-8bbf-9f0bc0a93a02	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 21:45:14.723443+07
193	c87443dc-b777-47b2-a1f0-e345c92e1b47	checkout.completed	order	5ff8f3cb-44a9-45da-a3ca-6e675945f212	\N	\N	\N	\N	{"payment_method": "wallet"}	2026-03-09 21:46:48.206756+07
194	0ee84d65-6576-4d63-b767-0359b06b8ec5	kyc_document.uploaded	kyc_documents	994f2fb1-5974-4360-a2a1-bce8cea742bc	\N	{"gcs_path": "gs://poool-assets-primary/kyc/0ee84d65-6576-4d63-b767-0359b06b8ec5/e9bc01bd-8ade-4aee-b88f-0fd2e051ed14.pdf", "document_type": "passport"}	\N	\N	\N	2026-03-09 23:13:39.387955+07
195	0ee84d65-6576-4d63-b767-0359b06b8ec5	kyc_document.uploaded	kyc_documents	0f8013c5-e9b2-4088-9f06-804629a769d0	\N	{"gcs_path": "gs://poool-assets-primary/kyc/0ee84d65-6576-4d63-b767-0359b06b8ec5/36c484ea-a16d-4841-9269-bef864fc19d0.pdf", "document_type": "passport"}	\N	\N	\N	2026-03-09 23:14:40.714849+07
196	0ee84d65-6576-4d63-b767-0359b06b8ec5	kyc_document.uploaded	kyc_documents	d8fd3b76-1abe-4122-af06-14129f6a0cb8	\N	{"gcs_path": "gs://poool-assets-primary/kyc/0ee84d65-6576-4d63-b767-0359b06b8ec5/2f6162ab-5c88-4588-a3a8-15ea004594d5.jpg", "document_type": "driving_licence"}	\N	\N	\N	2026-03-09 23:23:18.418115+07
197	0ee84d65-6576-4d63-b767-0359b06b8ec5	kyc_document.uploaded	kyc_documents	c1449707-d9f3-40f6-8236-6a75c3fc76a3	\N	{"gcs_path": "gs://poool-assets-primary/kyc/0ee84d65-6576-4d63-b767-0359b06b8ec5/1c8afe01-8b5c-4280-b6ef-0e00a3f86347.jpg", "document_type": "driving_licence"}	\N	\N	\N	2026-03-09 23:23:45.529019+07
\.


--
-- Data for Name: cart_items; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.cart_items (id, user_id, asset_id, tokens_quantity, token_price_cents, created_at, updated_at) FROM stdin;
00f70b6c-0ff8-4e8a-bb37-588d5ce73630	0ee84d65-6576-4d63-b767-0359b06b8ec5	e8719314-1fac-4e2a-ad2d-e86a725fec96	6989	18000	2026-03-09 23:24:41.69924+07	2026-03-09 23:24:50.695476+07
d3a6e852-b330-4134-9c82-463780c0df46	c87443dc-b777-47b2-a1f0-e345c92e1b47	e8719314-1fac-4e2a-ad2d-e86a725fec96	6989	18000	2026-03-09 23:34:03.837389+07	2026-03-09 23:34:25.77905+07
\.


--
-- Data for Name: deposit_requests; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.deposit_requests (id, user_id, currency, amount_cents, provider, provider_reference, status, payment_method, expires_at, paid_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: developer_projects; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.developer_projects (id, developer_id, asset_id, project_name, status, total_raised_cents, investors_count, funding_progress_bps, created_at, updated_at) FROM stdin;
b2222222-2222-2222-2222-222222222222	0ee84d65-6576-4d63-b767-0359b06b8ec5	a2222222-2222-2222-2222-222222222222	Central Plaza Commerce	in_review	0	0	0	2026-03-03 22:04:38.718487+07	2026-03-08 22:04:38.718487+07
b3333333-3333-3333-3333-333333333333	0ee84d65-6576-4d63-b767-0359b06b8ec5	a3333333-3333-3333-3333-333333333333	Green Field Agriculture	rejected	0	0	0	2026-02-26 22:04:38.718487+07	2026-03-08 22:04:38.718487+07
b4444444-4444-4444-4444-444444444444	0ee84d65-6576-4d63-b767-0359b06b8ec5	a4444444-4444-4444-4444-444444444444	Uluwatu Luxury Retreat	approved	250000000	42	10000	2026-02-21 22:04:38.718487+07	2026-03-08 22:04:38.718487+07
b1111111-1111-1111-1111-111111111111	0ee84d65-6576-4d63-b767-0359b06b8ec5	a1111111-1111-1111-1111-111111111111	Sunset Heights Villa	rejected	0	0	0	2026-03-06 22:04:38.718487+07	2026-03-08 23:55:29.993505+07
\.


--
-- Data for Name: dividend_payouts; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.dividend_payouts (id, investment_id, user_id, asset_id, amount_cents, payout_type, status, scheduled_at, paid_at, wallet_tx_id, created_at) FROM stdin;
408ab4a9-5fb3-4218-96ec-b26a87cdb5d7	d52b0f9f-9a26-4266-9a15-bdd40c9a3164	0ee84d65-6576-4d63-b767-0359b06b8ec5	89295282-6302-42d3-9961-a168b4578a40	4575	rental	paid	\N	2026-03-08 20:12:49.484753+07	\N	2026-03-08 20:12:49.484753+07
\.


--
-- Data for Name: email_logs; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.email_logs (id, user_id, template_id, subject, recipient_email, status, provider_id, error_message, sent_at, delivered_at, opened_at, clicked_at) FROM stdin;
1	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	You Have a New Notification	test@poool.app	queued	\N	{"document_type":"passport","provider":"didit"}	2026-03-09 01:45:22.625024+07	\N	\N	\N
2	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	You Have a New Notification	test@poool.app	queued	\N	{"document_type":"passport"}	2026-03-09 23:13:39.426461+07	\N	\N	\N
3	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	You Have a New Notification	test@poool.app	queued	\N	{"document_type":"passport"}	2026-03-09 23:14:40.756694+07	\N	\N	\N
\.


--
-- Data for Name: email_templates; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.email_templates (id, name, subject, html_template, text_template, version, description, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: investment_limits; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.investment_limits (id, user_id, annual_limit_cents, invested_12m_cents, limit_year, updated_at) FROM stdin;
0caf7c17-8162-4212-91f5-70ee3bd9d0e6	0ee84d65-6576-4d63-b767-0359b06b8ec5	50000000	2559500	2026	2026-03-08 20:12:49.484753+07
\.


--
-- Data for Name: investments; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.investments (id, user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, total_rental_cents, appreciation_pct_bps, status, payout_expected_at, purchased_at, updated_at) FROM stdin;
172ea98c-e0d9-4473-bb3e-ebaeaa87dc72	0ee84d65-6576-4d63-b767-0359b06b8ec5	6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771	30	345000	365000	22000	0	active	\N	2026-03-08 20:12:49.484753+07	2026-03-08 20:12:49.484753+07
d52b0f9f-9a26-4266-9a15-bdd40c9a3164	0ee84d65-6576-4d63-b767-0359b06b8ec5	89295282-6302-42d3-9961-a168b4578a40	100	950000	990000	91500	0	active	\N	2026-03-08 20:12:49.484753+07	2026-03-08 20:12:49.484753+07
f4c865c3-1bb4-436b-a1b9-21854dec97b9	0ee84d65-6576-4d63-b767-0359b06b8ec5	cb932002-0cdd-46e4-bb35-209e658060c9	75	487500	692250	0	0	exited	\N	2026-03-08 20:12:49.484753+07	2026-03-08 20:12:49.484753+07
90a966c6-b37a-493f-9602-c66ab82897b8	0ee84d65-6576-4d63-b767-0359b06b8ec5	fad407a3-b106-4903-bd4e-0772fc94c78e	10	50000	50000	0	0	active	\N	2026-03-08 20:12:49.484753+07	2026-03-08 20:12:49.484753+07
2fe23fc8-3d14-4e50-9ecd-553d77c1957c	0ee84d65-6576-4d63-b767-0359b06b8ec5	754e5be6-22f6-44fb-8cd7-f3aa90d86d25	20	60000	66600	0	0	active	\N	2026-03-08 20:12:49.484753+07	2026-03-08 20:12:49.484753+07
5be6a181-6d0d-4d82-adb5-d66a2d3044d2	c87443dc-b777-47b2-a1f0-e345c92e1b47	0a64f742-eb99-41f7-bd0a-9dd941b11011	998	13313320	13313320	0	0	funding_in_progress	\N	2026-03-09 20:47:13.568846+07	2026-03-09 20:47:13.568846+07
a7024692-c473-4c3a-87e2-1bd4ccde483c	c87443dc-b777-47b2-a1f0-e345c92e1b47	0081d064-996e-46a2-b19c-92a9d7389767	592	4736000	4736000	0	0	funding_in_progress	\N	2026-03-09 20:47:13.568846+07	2026-03-09 20:47:13.568846+07
83643f92-39e5-4ced-aa29-f0bd97854499	0ee84d65-6576-4d63-b767-0359b06b8ec5	0a64f742-eb99-41f7-bd0a-9dd941b11011	180	2401200	2434200	45000	0	active	\N	2026-03-08 20:12:49.484753+07	2026-03-09 21:45:14.723443+07
1089d0c7-a9ec-42be-9a77-f11c07ddadba	c87443dc-b777-47b2-a1f0-e345c92e1b47	e8719314-1fac-4e2a-ad2d-e86a725fec96	11	198000	198000	0	0	active	\N	2026-03-09 21:46:48.206756+07	2026-03-09 21:46:48.206756+07
\.


--
-- Data for Name: invoices; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.invoices (id, invoice_number, order_id, user_id, company_entity, subtotal_cents, tax_cents, total_cents, currency, pdf_url, status, notes, issued_at, created_at) FROM stdin;
4df385c3-069b-41cb-ab03-cee9c9d5d435	INV-2026-00001	b8dd1ce9-6105-42c1-8444-ca122165b940	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:03:11.247641+07	2026-03-09 02:03:11.247641+07
792f5a78-cc59-4483-aaa1-c0236142f45c	INV-2026-00002	e60bcc93-16f1-4024-8d57-07a2b17db01a	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:04:20.122872+07	2026-03-09 02:04:20.122872+07
79525c87-d452-4ecb-8c31-77f7590bdabf	INV-2026-00003	fc3e1ee2-db83-4b29-9d81-64dfda73a2c8	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:04:21.228762+07	2026-03-09 02:04:21.228762+07
e170ab11-d6a8-42bb-b42f-6768d5c79acc	INV-2026-00004	7e081384-8ab3-4bbd-ad4b-f0ef0f54f491	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:08:39.123654+07	2026-03-09 02:08:39.123654+07
7b088d16-a888-41b1-80af-190fc56e8d30	INV-2026-00005	68d0b021-ab3d-496a-8b75-08db3476718f	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:09:32.5834+07	2026-03-09 02:09:32.5834+07
004573fc-a007-4a4a-b5ad-d0d31446c9f5	INV-2026-00006	6fbc763b-0fb5-4323-8e48-60011a7982ea	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:09:35.221188+07	2026-03-09 02:09:35.221188+07
6c6c8d55-847d-4028-9004-0b972f678936	INV-2026-00007	0be5281d-777b-4dc5-81e5-b44de7a6425f	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:09:41.611415+07	2026-03-09 02:09:41.611415+07
1c332f1f-0d09-4b23-9304-065e317500cd	INV-2026-00008	cd7f1b23-1d87-46b8-973a-66e2ac450bd0	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:10:00.070503+07	2026-03-09 02:10:00.070503+07
23ce2852-b530-4c8f-97ca-4aeb30b10947	INV-2026-00009	ee7f4921-9f01-4d2a-a007-b152aae88916	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:10:08.253838+07	2026-03-09 02:10:08.253838+07
ff03755d-670a-45c4-8af7-9e094731d262	INV-2026-00010	6331c341-1a8c-4251-b5cc-06e47eb31b61	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:10:32.743319+07	2026-03-09 02:10:32.743319+07
dc2ee2e7-a94e-4082-93ce-e1908d634248	INV-2026-00011	26f45b02-9083-408e-bd81-3338ac99be17	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:12:49.510624+07	2026-03-09 02:12:49.510624+07
602ab67e-bc8a-4a68-9365-506a7467b137	INV-2026-00012	2c8aea8c-eda1-45da-b8cf-9d99360d5824	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 02:13:04.294602+07	2026-03-09 02:13:04.294602+07
3997aa35-99e9-427c-b6b6-cd605298b799	INV-2026-00013	5e571409-ab3d-4ba0-8d9b-1da600cd884a	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:12:26.956285+07	2026-03-09 10:12:26.956285+07
0b9b6cf1-8630-43cf-8fd0-7f16a985c46a	INV-2026-00014	1744c09a-3103-4f9d-83d1-34385835d063	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:16:18.62515+07	2026-03-09 10:16:18.62515+07
8fb7597d-3daa-4957-948b-d44e384cbcd0	INV-2026-00015	77e6b57f-041b-4b17-82a8-55d96f99e9d7	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:32:03.169033+07	2026-03-09 10:32:03.169033+07
e647d265-42f5-493a-863e-2a108af7a6ad	INV-2026-00016	ade11c77-0b08-490d-99f5-f2d2d89dd7f6	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:32:49.170805+07	2026-03-09 10:32:49.170805+07
d623fe95-1492-4308-bf42-f50bef3d640e	INV-2026-00017	ef05cc6c-6335-4a17-9c31-b58a9a836ee7	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:35:52.20138+07	2026-03-09 10:35:52.20138+07
53aca64f-59b0-4554-84b7-adc0dfa47933	INV-2026-00018	1dc1529f-f1e7-4361-9229-bab782f445ce	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:35:53.207412+07	2026-03-09 10:35:53.207412+07
f6d94540-1ba1-4d69-b665-93dc13c10fa0	INV-2026-00019	3b9c83e5-0922-47f9-9e39-58a26046fb65	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:35:58.926192+07	2026-03-09 10:35:58.926192+07
0ecb5024-371c-4621-b407-87cb72132631	INV-2026-00020	c4965f95-158f-4955-9dd1-3ec8e0963d8d	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:37:37.795949+07	2026-03-09 10:37:37.795949+07
bdb95c8d-67f8-4b68-9949-385a6cfa9710	INV-2026-00021	7edd981a-5f7c-4b2b-8c0c-56941b456962	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 10:37:53.548092+07	2026-03-09 10:37:53.548092+07
3ddaa0ac-6261-477a-a02c-2ab811e2ca3c	INV-2026-00022	f3aa5275-ccb0-4fb7-a871-c955768385ee	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 11:06:24.306505+07	2026-03-09 11:06:24.306505+07
63a2b943-3a76-46fe-9921-cb89c5caad95	INV-2026-00023	7f66ed20-7961-41ab-a432-8e03ffc7c97c	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 12:31:22.903322+07	2026-03-09 12:31:22.903322+07
fe820b72-1515-40d2-be2e-b9377e5c53cf	INV-2026-00024	e3fd0b07-b139-4f22-b903-2eeb4b2fbaa5	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 12:33:03.188747+07	2026-03-09 12:33:03.188747+07
31907aec-f35d-43b0-850d-59dd68dfec73	INV-2026-00025	19ab2def-ef5f-4a6e-a9c4-6e25f6c028b2	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:03:23.103631+07	2026-03-09 14:03:23.103631+07
472ad62c-d8a8-49ca-af0e-660791b26d29	INV-2026-00026	8b96ef77-caf8-4d9b-9ae8-37cc1fce554c	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:03:40.738182+07	2026-03-09 14:03:40.738182+07
78bc6675-c7ce-4b51-b2e7-5d46e7d62ed4	INV-2026-00027	88a38c6e-6433-4891-94be-ade8c8b27334	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:04:55.281419+07	2026-03-09 14:04:55.281419+07
32f0992b-956f-4bad-8cba-bec1b615595c	INV-2026-00028	19d63357-ee66-4958-b1a1-77c84d3b0b5e	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:08:05.963506+07	2026-03-09 14:08:05.963506+07
315777f6-ca0c-433d-bde9-4e36ca71c30c	INV-2026-00029	2a142c27-7e37-4f97-804b-52fab8a341ac	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:08:19.088165+07	2026-03-09 14:08:19.088165+07
364b0f19-20d7-48c1-b225-2ebb3c9e91b4	INV-2026-00030	b8bb9ea2-7306-48a2-b0f9-075c4c68db45	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:09:02.021988+07	2026-03-09 14:09:02.021988+07
e83b5e99-b96c-4a4b-a5fc-9aef56722ab7	INV-2026-00031	b040a9a2-fb69-4357-853d-e1d56a965393	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:10:15.913392+07	2026-03-09 14:10:15.913392+07
f1ba7fe9-9763-4450-b061-e856ee9e31d7	INV-2026-00032	2104199d-0fff-43cd-9def-27e998b79c25	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:12:31.729845+07	2026-03-09 14:12:31.729845+07
67d2ac6d-34c2-4655-bfd3-a918795887c9	INV-2026-00033	1fce8d81-c4c6-4639-a0a3-6150ca40638c	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:12:41.27867+07	2026-03-09 14:12:41.27867+07
5d645cf4-4526-4639-8cce-e42848828ecf	INV-2026-00034	87f34e8e-899c-488f-9d7a-93a2beecd2bc	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:13:04.234306+07	2026-03-09 14:13:04.234306+07
d5491192-1dcc-4fe3-ab4c-be13887fb506	INV-2026-00035	4df3ce86-2e20-4d12-9770-00354347ccb5	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:14:10.129231+07	2026-03-09 14:14:10.129231+07
ecdbc2af-6ab8-49a6-939b-986d4ed18306	INV-2026-00036	b0359c77-5557-451a-adcb-0a1077cfeadb	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:14:27.96879+07	2026-03-09 14:14:27.96879+07
471c8356-7f01-4dda-8b3b-360e5163cd10	INV-2026-00037	4c5a9699-e1bd-482d-904a-a6e21ec76959	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:15:04.096367+07	2026-03-09 14:15:04.096367+07
c24a5e9f-f922-425c-b08b-c80f5326d167	INV-2026-00038	05752b40-3625-4edc-a991-1f257734cb7b	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 14:15:44.972231+07	2026-03-09 14:15:44.972231+07
95cb0bd7-803c-4dbb-b055-4e3e698df222	INV-2026-00039	cf790796-a42d-494e-8239-de8a8709fcfc	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	13340	0	13340	USD	\N	issued	\N	2026-03-09 14:55:13.223497+07	2026-03-09 14:55:13.223497+07
1845c2ba-9c23-474a-9205-aa563eb6b475	INV-2026-00040	573260f7-fcbe-4193-8317-77c5f719f3f9	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	13340	0	13340	USD	\N	issued	\N	2026-03-09 14:55:39.82646+07	2026-03-09 14:55:39.82646+07
9b64e76d-3b66-4acd-94cc-ed35283d6d96	INV-2026-00041	33e2e6ab-9674-4948-ae20-cc0ab1670120	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 16:28:36.976888+07	2026-03-09 16:28:36.976888+07
3f8fd66d-8a8b-41ba-b43c-778313eb91bf	INV-2026-00042	6fa58552-b2db-4908-ace7-3cd55b600c40	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 16:29:17.644914+07	2026-03-09 16:29:17.644914+07
810a4028-3f4d-4344-b3af-4a7e8a5334ab	INV-2026-00043	3d19040e-c2c0-4fdb-908d-29e3d0e4ac23	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 18:15:17.378169+07	2026-03-09 18:15:17.378169+07
431a5a3b-1a3c-487f-a98c-3a8b8ab45db1	INV-2026-00044	e992d372-fec0-4041-a495-6d7b732415e3	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 19:28:31.785911+07	2026-03-09 19:28:31.785911+07
462b3d45-bee6-46fe-961f-4d622c734a13	INV-2026-00045	264fdd24-94e6-4261-9e53-b22d57d78ea6	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	26680	0	26680	USD	\N	issued	\N	2026-03-09 19:30:02.542928+07	2026-03-09 19:30:02.542928+07
0aab86b4-2055-4a4e-a8ff-0afe33781b35	INV-2026-00046	0ea0427a-84c4-4cda-acd6-51dc3321b1cd	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	186760	0	186760	USD	\N	issued	\N	2026-03-09 19:34:24.036683+07	2026-03-09 19:34:24.036683+07
a42afcfd-90d0-4ca2-9812-f6fbbae54422	INV-2026-00047	389b6003-0b0b-4f2d-af19-110099b874e4	c87443dc-b777-47b2-a1f0-e345c92e1b47	POOOL GmbH	18049320	0	18049320	USD	\N	issued	\N	2026-03-09 20:47:13.568846+07	2026-03-09 20:47:13.568846+07
fcd6979b-671a-4abc-8017-8e2c862eb064	INV-2026-00048	144ba81d-a451-4190-8bbf-9f0bc0a93a02	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL GmbH	373520	0	373520	USD	\N	issued	\N	2026-03-09 21:45:14.723443+07	2026-03-09 21:45:14.723443+07
35a8be9a-3d15-424a-a208-d5f839573e7a	INV-2026-00049	5ff8f3cb-44a9-45da-a3ca-6e675945f212	c87443dc-b777-47b2-a1f0-e345c92e1b47	POOOL GmbH	198000	0	198000	USD	\N	issued	\N	2026-03-09 21:46:48.206756+07	2026-03-09 21:46:48.206756+07
\.


--
-- Data for Name: kyc_documents; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.kyc_documents (id, user_id, document_type, gcs_path, status, rejection_reason, kyc_record_id, uploaded_at, reviewed_at) FROM stdin;
994f2fb1-5974-4360-a2a1-bce8cea742bc	0ee84d65-6576-4d63-b767-0359b06b8ec5	passport	gs://poool-assets-primary/kyc/0ee84d65-6576-4d63-b767-0359b06b8ec5/e9bc01bd-8ade-4aee-b88f-0fd2e051ed14.pdf	pending	\N	baae0c9e-da19-403b-a547-69d9d1bdd5d3	2026-03-09 23:13:39.375444+07	\N
0f8013c5-e9b2-4088-9f06-804629a769d0	0ee84d65-6576-4d63-b767-0359b06b8ec5	passport	gs://poool-assets-primary/kyc/0ee84d65-6576-4d63-b767-0359b06b8ec5/36c484ea-a16d-4841-9269-bef864fc19d0.pdf	pending	\N	cd43244d-ee33-466c-8b0e-470afd718a0e	2026-03-09 23:14:40.712144+07	\N
\.


--
-- Data for Name: kyc_records; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.kyc_records (id, user_id, provider, provider_ref_id, status, rejection_reason, document_type, pep_check_passed, sanctions_check, verified_at, expires_at, created_at, updated_at) FROM stdin;
ad6ae067-8111-438b-a7d1-8037332fa0fa	0ee84d65-6576-4d63-b767-0359b06b8ec5	sumsub	\N	rejected	delete	passport	\N	\N	\N	\N	2026-03-08 20:12:49.484753+07	2026-03-09 00:03:36.008244+07
2e6f40ff-4ebf-46e8-91a8-fc1d103d567f	0ee84d65-6576-4d63-b767-0359b06b8ec5	didit	5df07c17-810d-46b3-b846-8e8e43289838	approved	\N	passport	\N	\N	2026-03-09 10:20:01.300643+07	2028-03-09 10:20:01.300643+07	2026-03-09 01:45:22.620977+07	2026-03-09 10:20:01.300643+07
0366bafc-5d52-4bc8-b562-3d6cabc2c8b2	c87443dc-b777-47b2-a1f0-e345c92e1b47	manual	manual-admin-override	approved	\N	passport	\N	\N	2026-03-09 11:16:48.730962+07	2027-03-09 11:16:48.730962+07	2026-03-09 11:16:48.730962+07	2026-03-09 11:16:48.730962+07
094bb38d-0f1e-43ec-a674-50a2efe5ac3d	edc6dda5-e21d-420d-a088-003dbcb5a827	sumsub	\N	approved	\N	\N	\N	\N	\N	\N	2026-03-09 16:40:38.723722+07	2026-03-09 16:40:38.723722+07
8825cf98-379c-4720-a9ec-694f7bbf5b59	3f16bd25-2aff-42df-b193-bbc3faffb12b	sumsub	\N	approved	\N	\N	\N	\N	\N	\N	2026-03-09 16:41:06.089534+07	2026-03-09 16:41:06.089534+07
baae0c9e-da19-403b-a547-69d9d1bdd5d3	0ee84d65-6576-4d63-b767-0359b06b8ec5	manual	\N	pending	\N	passport	\N	\N	\N	\N	2026-03-09 23:13:39.4211+07	2026-03-09 23:13:39.4211+07
cd43244d-ee33-466c-8b0e-470afd718a0e	0ee84d65-6576-4d63-b767-0359b06b8ec5	manual	\N	pending	\N	passport	\N	\N	\N	\N	2026-03-09 23:14:40.753474+07	2026-03-09 23:14:40.753474+07
\.


--
-- Data for Name: notifications; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.notifications (id, user_id, title, message, type, is_read, action_url, created_at) FROM stdin;
c64dddf0-fb31-4651-bd02-d7a910fd1988	0ee84d65-6576-4d63-b767-0359b06b8ec5	Investment Confirmed	Your investment of $6,670 in Luxury Clifftop Villa has been confirmed.	investment	t	\N	2026-03-08 20:12:49.484753+07
84bf65ff-8c48-446f-82d4-64eb8e3d8f17	0ee84d65-6576-4d63-b767-0359b06b8ec5	Investment Confirmed	Your investment of $3,450 in Modern Surf Villa has been confirmed.	investment	t	\N	2026-03-08 20:12:49.484753+07
c638bbdb-9ef4-4b57-b540-9b2966dddc2a	0ee84d65-6576-4d63-b767-0359b06b8ec5	Welcome to POOOL!	Welcome aboard! Start exploring investment opportunities on the marketplace.	system	t	\N	2026-03-08 20:12:49.484753+07
6806e38b-9269-4829-a1a8-945a7230eccf	0ee84d65-6576-4d63-b767-0359b06b8ec5	Monthly Payout	You received a $45.75 dividend payout from Luxury Pool Villa.	payout	f	\N	2026-03-08 20:12:49.484753+07
04feabe2-4fdf-462b-9fb1-df23e8fd59d7	0ee84d65-6576-4d63-b767-0359b06b8ec5	KYC Required	Please complete your identity verification to unlock all features.	system	f	\N	2026-03-08 20:12:49.484753+07
1d55fc0b-b61e-49b4-ab7a-51001439a073	0ee84d65-6576-4d63-b767-0359b06b8ec5	Property Funded!	Luxury Pool Villa has been fully funded. Congratulations!	investment	t	\N	2026-03-08 20:12:49.484753+07
e6fcfe0d-aca0-45ac-908d-ef29ae9dfc60	0ee84d65-6576-4d63-b767-0359b06b8ec5	New Property Listed	A new investment opportunity "Renovation Flip Project" is now available.	system	f	\N	2026-03-08 20:12:49.484753+07
0f191611-ad9f-4e7a-a7e8-e381aa528352	0ee84d65-6576-4d63-b767-0359b06b8ec5	Project Approved! 🎉	Congratulations! Your project "Sunset Heights Villa" has been approved and is now live on the marketplace.	investment	f	/developer/assets	2026-03-08 23:54:26.160358+07
6665427c-79d5-43af-9850-462a1c0ff000	0ee84d65-6576-4d63-b767-0359b06b8ec5	Project Approved! 🎉	Congratulations! Your project "Sunset Heights Villa" has been approved and is now live on the marketplace.	investment	f	/developer/assets	2026-03-08 23:54:29.405003+07
0644f116-716b-468f-9b82-e718eecfeb86	0ee84d65-6576-4d63-b767-0359b06b8ec5	Project Approved! 🎉	Congratulations! Your project "Sunset Heights Villa" has been approved and is now live on the marketplace.	investment	f	/developer/assets	2026-03-08 23:55:19.638654+07
2650c98f-d710-4e24-a4fa-b9fa98bc9d4a	0ee84d65-6576-4d63-b767-0359b06b8ec5	Project Submission Rejected	Your project "Sunset Heights Villa" has been rejected. Reason: deete. Please contact support if you have questions.	system	f	/developer/assets	2026-03-08 23:55:29.996811+07
b813005a-30e8-484e-a960-bdec79292804	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	test	test	promo	f	\N	2026-03-09 14:28:26.502062+07
0da61d0b-37a6-4e03-9452-29a6c2fb6526	f91c696d-f3e2-4525-80e1-01a29d700062	test	test	promo	f	\N	2026-03-09 14:28:26.502062+07
3f6b07df-2d8e-46d9-b6f0-aa3f6c6b7fe1	db7174f5-c055-470c-ad50-b969ccfea5a4	test	test	promo	f	\N	2026-03-09 14:28:26.502062+07
00189ff3-44f8-4570-8edf-f5128cda8aa6	dee2990f-8070-4d34-ab17-e7fb2c041b8b	test	test	promo	f	\N	2026-03-09 14:28:26.502062+07
8bb3051a-22c0-42b4-9f10-ffe9e8e865fc	0ee84d65-6576-4d63-b767-0359b06b8ec5	test	test	promo	f	\N	2026-03-09 14:28:26.502062+07
cd1c3b9f-7cf4-48ba-ae9f-14efc7eab7f0	b45f038b-3e75-41e7-806a-0db9ca5b9272	test	test	promo	f	\N	2026-03-09 14:28:26.502062+07
687a2032-5054-4ffb-8975-e039edf69c32	c87443dc-b777-47b2-a1f0-e345c92e1b47	test	test	promo	f	\N	2026-03-09 14:28:26.502062+07
\.


--
-- Data for Name: oauth_accounts; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.oauth_accounts (id, user_id, provider, provider_id, provider_email, access_token, refresh_token, created_at) FROM stdin;
\.


--
-- Data for Name: order_items; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.order_items (id, order_id, asset_id, tokens_quantity, token_price_cents, subtotal_cents) FROM stdin;
d150f896-f8ca-4cfa-afe4-abde5fc79b92	b8dd1ce9-6105-42c1-8444-ca122165b940	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
686319c6-2be9-4bff-acb6-9e7898a607e4	e60bcc93-16f1-4024-8d57-07a2b17db01a	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
dc69741c-0cc9-4d89-b112-6c048d146809	fc3e1ee2-db83-4b29-9d81-64dfda73a2c8	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
28b0865b-2f6e-4a79-a426-4bff1113c7fc	7e081384-8ab3-4bbd-ad4b-f0ef0f54f491	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
893c62b8-ba10-4cff-9d98-0cd15982ca6d	68d0b021-ab3d-496a-8b75-08db3476718f	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
346b7ea0-b58c-487c-9096-9112d4a6fa1a	6fbc763b-0fb5-4323-8e48-60011a7982ea	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
a0c491da-f2d2-4e22-b639-76dc2acacf81	0be5281d-777b-4dc5-81e5-b44de7a6425f	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
b6025249-9ce0-4b8e-9f9c-133356e3660a	cd7f1b23-1d87-46b8-973a-66e2ac450bd0	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
65a10b7b-15e4-478e-8e2f-6d7c7dcb1964	ee7f4921-9f01-4d2a-a007-b152aae88916	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
a066e051-a31e-4210-8e02-a8d53a138395	6331c341-1a8c-4251-b5cc-06e47eb31b61	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
91ee5368-8b86-4b81-98a9-f917e81e9ebb	26f45b02-9083-408e-bd81-3338ac99be17	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
0dabe1e6-1389-47e8-9448-868d29470e49	2c8aea8c-eda1-45da-b8cf-9d99360d5824	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
e465e35f-b0d1-4f9c-bf15-937072b5ae2a	5e571409-ab3d-4ba0-8d9b-1da600cd884a	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
c29d6a85-d308-477c-8d6c-c6bf87ac48d9	1744c09a-3103-4f9d-83d1-34385835d063	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
254c03a4-819c-464a-998a-688df2fefd3d	77e6b57f-041b-4b17-82a8-55d96f99e9d7	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
d47235a0-e821-4f32-b98f-1d5a4c29bd3d	ade11c77-0b08-490d-99f5-f2d2d89dd7f6	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
4b07a5f2-1d6b-4fe0-b327-7c695080f0f2	ef05cc6c-6335-4a17-9c31-b58a9a836ee7	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
ff168b1f-a0ed-415f-9465-6d9573143700	1dc1529f-f1e7-4361-9229-bab782f445ce	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
e37ef01e-bf7b-47e8-99b2-d4e531dd3619	3b9c83e5-0922-47f9-9e39-58a26046fb65	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
524d0883-ab5b-4750-b3de-9a8b0fc3f17f	c4965f95-158f-4955-9dd1-3ec8e0963d8d	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
d3575e86-acf7-43c0-90e2-1abcda69e0f8	7edd981a-5f7c-4b2b-8c0c-56941b456962	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
5cbb93d2-e375-49f1-a38c-7657662ac7f7	f3aa5275-ccb0-4fb7-a871-c955768385ee	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
e74e00fe-326f-4676-8bf0-e47c81ef1b44	7f66ed20-7961-41ab-a432-8e03ffc7c97c	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
3d8ebc60-3a67-4b1f-abef-812c225c5185	e3fd0b07-b139-4f22-b903-2eeb4b2fbaa5	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
637b1d23-d97c-41ee-a46f-46770410f65a	19ab2def-ef5f-4a6e-a9c4-6e25f6c028b2	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
2588d706-e5f0-4e6b-86f4-0f889469b694	8b96ef77-caf8-4d9b-9ae8-37cc1fce554c	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
fd750188-5cd7-4b17-8189-101e3a7a7ca7	88a38c6e-6433-4891-94be-ade8c8b27334	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
9bd4791d-95a4-4297-9eda-8cb25b1ec4f1	19d63357-ee66-4958-b1a1-77c84d3b0b5e	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
379441ac-9855-4956-bdc2-599cc224d142	2a142c27-7e37-4f97-804b-52fab8a341ac	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
42c764e6-8bb1-4443-8bd6-6a8c6f0c6989	b8bb9ea2-7306-48a2-b0f9-075c4c68db45	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
fbb706df-8abf-4a6e-9c8f-9bf8e54fee35	b040a9a2-fb69-4357-853d-e1d56a965393	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
69c36a00-acea-422d-b9a7-46d8eb5a784f	2104199d-0fff-43cd-9def-27e998b79c25	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
0db04463-37b0-431c-bad5-e1945e230c3b	1fce8d81-c4c6-4639-a0a3-6150ca40638c	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
51c858e4-318b-44ca-9aa0-61441245245e	87f34e8e-899c-488f-9d7a-93a2beecd2bc	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
48d13646-45ad-4125-a876-873589b55a6d	4df3ce86-2e20-4d12-9770-00354347ccb5	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
d47819cc-7cdc-4559-8d6d-b5c3b171aca5	b0359c77-5557-451a-adcb-0a1077cfeadb	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
292028a1-ca47-4d09-ba10-b38759cb35fb	4c5a9699-e1bd-482d-904a-a6e21ec76959	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
7ae31f7b-225d-420f-b078-b13db8719104	05752b40-3625-4edc-a991-1f257734cb7b	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
a95dd513-2e03-4272-b115-3d6c6e944597	cf790796-a42d-494e-8239-de8a8709fcfc	0a64f742-eb99-41f7-bd0a-9dd941b11011	1	13340	13340
1bbbdc2f-9ac4-4297-aed4-015f4c40bb53	573260f7-fcbe-4193-8317-77c5f719f3f9	0a64f742-eb99-41f7-bd0a-9dd941b11011	1	13340	13340
8bdf0b21-7691-4fd1-b36e-db9679eeefd7	33e2e6ab-9674-4948-ae20-cc0ab1670120	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
7dd35ff9-f736-4215-ab1f-b71e682d7585	6fa58552-b2db-4908-ace7-3cd55b600c40	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
8de5a9e4-bccb-4f79-8d70-b9ca56560a12	3d19040e-c2c0-4fdb-908d-29e3d0e4ac23	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
daecf90d-52b5-49a9-8f1e-85f907d3f319	e992d372-fec0-4041-a495-6d7b732415e3	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
3a682b55-95e2-4476-862e-c6a39c79038f	264fdd24-94e6-4261-9e53-b22d57d78ea6	0a64f742-eb99-41f7-bd0a-9dd941b11011	2	13340	26680
74c10e7c-645b-4628-a844-9d503d8674a1	0ea0427a-84c4-4cda-acd6-51dc3321b1cd	0a64f742-eb99-41f7-bd0a-9dd941b11011	14	13340	186760
9e629900-9fac-476d-9831-e1178c9af9c3	389b6003-0b0b-4f2d-af19-110099b874e4	0a64f742-eb99-41f7-bd0a-9dd941b11011	998	13340	13313320
07955447-a99e-4f02-a039-ded45c350027	389b6003-0b0b-4f2d-af19-110099b874e4	0081d064-996e-46a2-b19c-92a9d7389767	592	8000	4736000
c47f4bc9-b9cf-43f2-95b3-db10dab60564	144ba81d-a451-4190-8bbf-9f0bc0a93a02	0a64f742-eb99-41f7-bd0a-9dd941b11011	28	13340	373520
d1473314-ecb5-459e-aacf-7a553660e553	5ff8f3cb-44a9-45da-a3ca-6e675945f212	e8719314-1fac-4e2a-ad2d-e86a725fec96	11	18000	198000
\.


--
-- Data for Name: orders; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.orders (id, user_id, order_number, total_cents, status, payment_method, payment_ref_id, created_at, completed_at, currency, payment_currency, fx_rate, fx_provider) FROM stdin;
e70fd3ee-f290-428b-9864-25d0e815b757	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-2026-0001	667000	completed	wallet	\N	2026-03-08 20:12:49.484753+07	2026-01-22 20:12:49.484753+07	USD	\N	\N	\N
0a17a81e-0f85-4698-8489-fd6ff0614c59	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-2026-0002	345000	completed	wallet	\N	2026-03-08 20:12:49.484753+07	2026-02-06 20:12:49.484753+07	USD	\N	\N	\N
b2f6451d-f1b7-469c-a8da-7e19f1eca902	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-2026-0003	950000	completed	wallet	\N	2026-03-08 20:12:49.484753+07	2026-02-21 20:12:49.484753+07	USD	\N	\N	\N
b8dd1ce9-6105-42c1-8444-ca122165b940	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308190311	26680	completed	wallet	\N	2026-03-09 02:03:11.247641+07	2026-03-09 02:03:11.248713+07	USD	USD	\N	\N
e60bcc93-16f1-4024-8d57-07a2b17db01a	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308190420	26680	completed	wallet	\N	2026-03-09 02:04:20.122872+07	2026-03-09 02:04:20.124204+07	USD	USD	\N	\N
fc3e1ee2-db83-4b29-9d81-64dfda73a2c8	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308190421	26680	completed	wallet	\N	2026-03-09 02:04:21.228762+07	2026-03-09 02:04:21.229941+07	USD	USD	\N	\N
7e081384-8ab3-4bbd-ad4b-f0ef0f54f491	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308190839	26680	completed	wallet	\N	2026-03-09 02:08:39.123654+07	2026-03-09 02:08:39.125463+07	USD	USD	\N	\N
68d0b021-ab3d-496a-8b75-08db3476718f	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308190932	26680	completed	wallet	\N	2026-03-09 02:09:32.5834+07	2026-03-09 02:09:32.584635+07	USD	USD	\N	\N
6fbc763b-0fb5-4323-8e48-60011a7982ea	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308190935	26680	completed	wallet	\N	2026-03-09 02:09:35.221188+07	2026-03-09 02:09:35.222871+07	USD	USD	\N	\N
0be5281d-777b-4dc5-81e5-b44de7a6425f	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308190941	26680	completed	wallet	\N	2026-03-09 02:09:41.611415+07	2026-03-09 02:09:41.612479+07	USD	USD	\N	\N
cd7f1b23-1d87-46b8-973a-66e2ac450bd0	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308191000	26680	completed	wallet	\N	2026-03-09 02:10:00.070503+07	2026-03-09 02:10:00.07154+07	USD	USD	\N	\N
ee7f4921-9f01-4d2a-a007-b152aae88916	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308191008	26680	completed	wallet	\N	2026-03-09 02:10:08.253838+07	2026-03-09 02:10:08.254809+07	USD	USD	\N	\N
6331c341-1a8c-4251-b5cc-06e47eb31b61	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308191032	26680	completed	wallet	\N	2026-03-09 02:10:32.743319+07	2026-03-09 02:10:32.746122+07	USD	USD	\N	\N
26f45b02-9083-408e-bd81-3338ac99be17	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308191249	26680	completed	wallet	\N	2026-03-09 02:12:49.510624+07	2026-03-09 02:12:49.511871+07	USD	USD	\N	\N
2c8aea8c-eda1-45da-b8cf-9d99360d5824	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260308191304	26680	completed	wallet	\N	2026-03-09 02:13:04.294602+07	2026-03-09 02:13:04.295824+07	USD	USD	\N	\N
5e571409-ab3d-4ba0-8d9b-1da600cd884a	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309031226	26680	completed	wallet	\N	2026-03-09 10:12:26.956285+07	2026-03-09 10:12:26.957318+07	USD	USD	\N	\N
1744c09a-3103-4f9d-83d1-34385835d063	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309031618	26680	completed	wallet	\N	2026-03-09 10:16:18.62515+07	2026-03-09 10:16:18.627657+07	USD	USD	\N	\N
77e6b57f-041b-4b17-82a8-55d96f99e9d7	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309033203	26680	completed	wallet	\N	2026-03-09 10:32:03.169033+07	2026-03-09 10:32:03.17331+07	USD	USD	\N	\N
ade11c77-0b08-490d-99f5-f2d2d89dd7f6	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309033249	26680	completed	wallet	\N	2026-03-09 10:32:49.170805+07	2026-03-09 10:32:49.171976+07	USD	USD	\N	\N
ef05cc6c-6335-4a17-9c31-b58a9a836ee7	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309033552	26680	completed	wallet	\N	2026-03-09 10:35:52.20138+07	2026-03-09 10:35:52.203226+07	USD	USD	\N	\N
1dc1529f-f1e7-4361-9229-bab782f445ce	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309033553	26680	completed	wallet	\N	2026-03-09 10:35:53.207412+07	2026-03-09 10:35:53.20821+07	USD	USD	\N	\N
3b9c83e5-0922-47f9-9e39-58a26046fb65	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309033558	26680	completed	wallet	\N	2026-03-09 10:35:58.926192+07	2026-03-09 10:35:58.927458+07	USD	USD	\N	\N
c4965f95-158f-4955-9dd1-3ec8e0963d8d	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309033737	26680	completed	wallet	\N	2026-03-09 10:37:37.795949+07	2026-03-09 10:37:37.797316+07	USD	USD	\N	\N
7edd981a-5f7c-4b2b-8c0c-56941b456962	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309033753	26680	completed	wallet	\N	2026-03-09 10:37:53.548092+07	2026-03-09 10:37:53.548704+07	USD	USD	\N	\N
f3aa5275-ccb0-4fb7-a871-c955768385ee	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309040624	26680	completed	wallet	\N	2026-03-09 11:06:24.306505+07	2026-03-09 11:06:24.308141+07	USD	USD	\N	\N
7f66ed20-7961-41ab-a432-8e03ffc7c97c	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309053122	26680	completed	wallet	\N	2026-03-09 12:31:22.903322+07	2026-03-09 12:31:22.904688+07	USD	USD	\N	\N
e3fd0b07-b139-4f22-b903-2eeb4b2fbaa5	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309053303	26680	completed	wallet	\N	2026-03-09 12:33:03.188747+07	2026-03-09 12:33:03.189398+07	USD	USD	\N	\N
19ab2def-ef5f-4a6e-a9c4-6e25f6c028b2	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309070323	26680	completed	wallet	\N	2026-03-09 14:03:23.103631+07	2026-03-09 14:03:23.106713+07	USD	USD	\N	\N
8b96ef77-caf8-4d9b-9ae8-37cc1fce554c	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309070340	26680	completed	wallet	\N	2026-03-09 14:03:40.738182+07	2026-03-09 14:03:40.740328+07	USD	USD	\N	\N
88a38c6e-6433-4891-94be-ade8c8b27334	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309070455	26680	completed	wallet	\N	2026-03-09 14:04:55.281419+07	2026-03-09 14:04:55.283275+07	USD	USD	\N	\N
19d63357-ee66-4958-b1a1-77c84d3b0b5e	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309070805	26680	completed	wallet	\N	2026-03-09 14:08:05.963506+07	2026-03-09 14:08:05.965405+07	USD	USD	\N	\N
2a142c27-7e37-4f97-804b-52fab8a341ac	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309070819	26680	completed	wallet	\N	2026-03-09 14:08:19.088165+07	2026-03-09 14:08:19.089229+07	USD	USD	\N	\N
b8bb9ea2-7306-48a2-b0f9-075c4c68db45	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309070902	26680	completed	wallet	\N	2026-03-09 14:09:02.021988+07	2026-03-09 14:09:02.023319+07	USD	USD	\N	\N
b040a9a2-fb69-4357-853d-e1d56a965393	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071015	26680	completed	wallet	\N	2026-03-09 14:10:15.913392+07	2026-03-09 14:10:15.915642+07	USD	USD	\N	\N
2104199d-0fff-43cd-9def-27e998b79c25	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071231	26680	completed	wallet	\N	2026-03-09 14:12:31.729845+07	2026-03-09 14:12:31.731147+07	USD	USD	\N	\N
1fce8d81-c4c6-4639-a0a3-6150ca40638c	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071241	26680	completed	wallet	\N	2026-03-09 14:12:41.27867+07	2026-03-09 14:12:41.279263+07	USD	USD	\N	\N
87f34e8e-899c-488f-9d7a-93a2beecd2bc	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071304	26680	completed	wallet	\N	2026-03-09 14:13:04.234306+07	2026-03-09 14:13:04.235688+07	USD	USD	\N	\N
4df3ce86-2e20-4d12-9770-00354347ccb5	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071410	26680	completed	wallet	\N	2026-03-09 14:14:10.129231+07	2026-03-09 14:14:10.134813+07	USD	USD	\N	\N
b0359c77-5557-451a-adcb-0a1077cfeadb	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071427	26680	completed	wallet	\N	2026-03-09 14:14:27.96879+07	2026-03-09 14:14:27.970087+07	USD	USD	\N	\N
4c5a9699-e1bd-482d-904a-a6e21ec76959	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071504	26680	completed	wallet	\N	2026-03-09 14:15:04.096367+07	2026-03-09 14:15:04.097563+07	USD	USD	\N	\N
05752b40-3625-4edc-a991-1f257734cb7b	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309071544	26680	completed	wallet	\N	2026-03-09 14:15:44.972231+07	2026-03-09 14:15:44.973225+07	USD	USD	\N	\N
cf790796-a42d-494e-8239-de8a8709fcfc	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309075513	13340	completed	wallet	\N	2026-03-09 14:55:13.223497+07	2026-03-09 14:55:13.225984+07	USD	USD	\N	\N
573260f7-fcbe-4193-8317-77c5f719f3f9	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309075539	13340	completed	wallet	\N	2026-03-09 14:55:39.82646+07	2026-03-09 14:55:39.829481+07	USD	USD	\N	\N
33e2e6ab-9674-4948-ae20-cc0ab1670120	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309092836	26680	completed	wallet	\N	2026-03-09 16:28:36.976888+07	2026-03-09 16:28:36.97852+07	USD	USD	\N	\N
6fa58552-b2db-4908-ace7-3cd55b600c40	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309092917	26680	completed	wallet	\N	2026-03-09 16:29:17.644914+07	2026-03-09 16:29:17.645553+07	USD	USD	\N	\N
3d19040e-c2c0-4fdb-908d-29e3d0e4ac23	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309111517	26680	completed	wallet	\N	2026-03-09 18:15:17.378169+07	2026-03-09 18:15:17.382097+07	USD	USD	\N	\N
e992d372-fec0-4041-a495-6d7b732415e3	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309122831	26680	completed	wallet	\N	2026-03-09 19:28:31.785911+07	2026-03-09 19:28:31.789042+07	USD	USD	\N	\N
264fdd24-94e6-4261-9e53-b22d57d78ea6	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309123002	26680	completed	wallet	\N	2026-03-09 19:30:02.542928+07	2026-03-09 19:30:02.547881+07	USD	USD	\N	\N
0ea0427a-84c4-4cda-acd6-51dc3321b1cd	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309123424	186760	pending	bank	\N	2026-03-09 19:34:24.036683+07	\N	USD	USD	\N	\N
389b6003-0b0b-4f2d-af19-110099b874e4	c87443dc-b777-47b2-a1f0-e345c92e1b47	ORD-20260309134713	18049320	pending	bank	\N	2026-03-09 20:47:13.568846+07	\N	USD	USD	\N	\N
144ba81d-a451-4190-8bbf-9f0bc0a93a02	0ee84d65-6576-4d63-b767-0359b06b8ec5	ORD-20260309144514	373520	completed	wallet	\N	2026-03-09 21:45:14.723443+07	2026-03-09 21:45:14.733944+07	USD	USD	\N	\N
5ff8f3cb-44a9-45da-a3ca-6e675945f212	c87443dc-b777-47b2-a1f0-e345c92e1b47	ORD-20260309144648	198000	completed	wallet	\N	2026-03-09 21:46:48.206756+07	2026-03-09 21:46:48.211165+07	USD	USD	\N	\N
\.


--
-- Data for Name: password_reset_tokens; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.password_reset_tokens (id, user_id, token_hash, expires_at, used_at, created_at) FROM stdin;
\.


--
-- Data for Name: payment_methods; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.payment_methods (id, user_id, method_type, processor_type, processor_token, customer_id, brand, last_four, expiry_month, expiry_year, holder_name, routing_number, bank_country, label, is_default, status, created_at, updated_at) FROM stdin;
689e0152-8c03-4423-b4ee-6ce6312dbf1a	0ee84d65-6576-4d63-b767-0359b06b8ec5	card	stripe	pm_mock_123	\N	Visa	4242	\N	\N	Test User	\N	\N	\N	t	active	2026-03-08 23:31:25.990833+07	2026-03-08 23:31:25.990833+07
\.


--
-- Data for Name: platform_settings; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.platform_settings (key, value, value_type, description, updated_at, updated_by) FROM stdin;
platform_name	POOOL Finance	string	Display name used across the platform	2026-03-08 20:10:50.530409+07	\N
support_email	support@poool.finance	string	Support contact email	2026-03-08 20:10:50.530409+07	\N
enable_registrations	true	boolean	Allow new user registrations	2026-03-08 20:10:50.530409+07	\N
require_kyc	true	boolean	Require KYC verification for investments	2026-03-08 20:10:50.530409+07	\N
platform_fee_percent	2.50	number	Platform fee percentage on token purchases	2026-03-08 20:10:50.530409+07	\N
withdrawal_fee_cents	500	number	Flat withdrawal fee in cents	2026-03-08 20:10:50.530409+07	\N
referral_commission_percent	1.00	number	Referral commission percentage	2026-03-08 20:10:50.530409+07	\N
min_withdrawal_cents	1000	number	Minimum withdrawal amount in cents	2026-03-08 20:10:50.530409+07	\N
maintenance_mode	false	boolean	Redirect all users to maintenance page	2026-03-08 20:10:50.530409+07	\N
resend_api_key		string	API key for Resend email provider	2026-03-08 20:10:50.569374+07	\N
legal_terms_version	1.0	string	Current active Terms & Conditions version. Increment to prompt all users to re-accept.	2026-03-08 21:28:18.155522+07	\N
legal_privacy_version	1.0	string	Current active Privacy Policy version.	2026-03-08 21:28:18.155522+07	\N
legal_last_updated	2026-03-08	string	Date when legal documents were last updated (YYYY-MM-DD).	2026-03-08 21:28:18.155522+07	\N
\.


--
-- Data for Name: referral_clicks; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.referral_clicks (id, code, ip_address, user_agent, created_at, subid) FROM stdin;
\.


--
-- Data for Name: referral_codes; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.referral_codes (id, user_id, code, created_at) FROM stdin;
a6cbc0a9-3eeb-4c24-8159-0d35952c7888	c87443dc-b777-47b2-a1f0-e345c92e1b47	d8e07596	2026-03-08 21:34:57.540277+07
cc4190c2-1f00-476a-ae83-07785a0612f1	f91c696d-f3e2-4525-80e1-01a29d700062	2ac7751d	2026-03-08 22:12:22.157542+07
f70095ce-f478-4ba4-ac6b-4bf5a36d5a0f	db7174f5-c055-470c-ad50-b969ccfea5a4	3ea096eb	2026-03-08 22:12:46.974195+07
af7234be-d334-4813-af7b-d9d01480d0c2	0ee84d65-6576-4d63-b767-0359b06b8ec5	POOOL001	2026-03-08 22:11:59.806677+07
\.


--
-- Data for Name: referral_tracking; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.referral_tracking (id, referrer_id, referred_id, status, referrer_reward, referred_reward, created_at, qualified_at, subid) FROM stdin;
0a1850b0-4792-4014-8374-7034435adb11	0ee84d65-6576-4d63-b767-0359b06b8ec5	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	paid	2500	500	2026-03-08 22:12:00.175481+07	2026-03-08 22:12:00.270742+07	\N
69871a71-cad7-4bcd-9b48-74f0cca8e968	0ee84d65-6576-4d63-b767-0359b06b8ec5	f91c696d-f3e2-4525-80e1-01a29d700062	paid	2500	500	2026-03-08 22:12:20.978849+07	2026-03-08 22:12:21.063683+07	\N
eb5fc1b3-97f4-48e2-ae96-d034cb9c7243	0ee84d65-6576-4d63-b767-0359b06b8ec5	db7174f5-c055-470c-ad50-b969ccfea5a4	paid	2500	500	2026-03-08 22:12:45.790828+07	2026-03-08 22:12:45.878585+07	\N
\.


--
-- Data for Name: rewards_balances; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.rewards_balances (id, user_id, cashback, referrals, promotions, updated_at) FROM stdin;
71247e97-fdf4-4284-8ca9-6e81d68a4fdc	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	0	500	0	2026-03-08 22:12:00.273683+07
2d8e25b9-ad59-4fa0-8ae2-c12fa703cdd8	f91c696d-f3e2-4525-80e1-01a29d700062	0	500	0	2026-03-08 22:12:21.064696+07
9b06d0f6-0302-4c1f-8c92-5c545fa5c896	db7174f5-c055-470c-ad50-b969ccfea5a4	0	500	0	2026-03-08 22:12:45.879633+07
f4828e7e-0d06-4528-ac74-7ecfb18253f9	0ee84d65-6576-4d63-b767-0359b06b8ec5	105000	4000	12000	2026-03-09 01:59:02.553332+07
\.


--
-- Data for Name: roles; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.roles (id, name, description) FROM stdin;
4f09bcb9-17e1-4608-bf21-f950fae5733a	investor	Standard-Investor mit Zugang zum Marketplace und Portfolio
21c53943-2c69-407d-9f90-eed6dafd23b9	developer	Immobilien-Entwickler, der Assets einstellen kann
348274de-230c-4ed5-8c5f-6ec0952d46f0	admin	Plattform-Administrator
3368e3a7-10ac-4e20-9c3b-015956bddca0	super_admin	Super administrator with full access
71b01f28-a9b5-4918-9f99-942e58c2b5e3	compliance	Compliance officer - KYC and AML access
271b2cb4-0d3f-40a6-a402-9a4c61d1c158	support	Support agent - ticket management
99ccc83b-f91f-4409-8662-8e493249f4a7	finance	Finance manager - treasury and payouts
\.


--
-- Data for Name: support_ticket_attachments; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.support_ticket_attachments (id, reply_id, file_url, file_type, file_size_bytes, created_at) FROM stdin;
\.


--
-- Data for Name: support_ticket_replies; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.support_ticket_replies (id, ticket_id, author_id, author_name, author_role, type, content, created_at) FROM stdin;
22cc40e4-ca85-4895-9501-3d63fb242e6e	21becfa7-07b1-4675-b2b9-d8a6d7e82335	0ee84d65-6576-4d63-b767-0359b06b8ec5	test@poool.app	user	initial	I tried uploading my passport but it keeps failing with a generic error. I've tried 3 times now.	2026-03-06 20:13:00.479341+07
a45fabcb-456e-459d-874f-be7237f85490	faacae8b-7e15-468e-a284-5528650e024c	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	user	initial	My withdrawal to my bank account is still pending. Usually it takes 24h. Can you please check?	2026-03-06 20:13:00.481516+07
fc3804b7-ac90-4a93-96f8-d8fa4a846bda	faacae8b-7e15-468e-a284-5528650e024c	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	admin	reply	Hello, we are checking this with our payment provider. There's a slight delay in processing SEPA transfers today.	2026-03-06 22:13:00.481516+07
4c753903-d320-4245-874e-e50f4952a937	faacae8b-7e15-468e-a284-5528650e024c	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	user	reply	Okay, thank you for the update. Do you have an ETA? I need the funds by Friday.	2026-03-07 00:13:00.481516+07
67ff8303-a78c-4718-922c-327f88df5517	faacae8b-7e15-468e-a284-5528650e024c	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	admin	reply	We expect it to be cleared by tomorrow morning. We've prioritized your request.	2026-03-07 02:13:00.481516+07
e9cb53d7-72ab-4645-8a34-22dd4af676d5	c487abc6-cc16-4633-b71f-032780764e3f	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	user	initial	I want to invest more than $10,000. How can I increase my limit? Is there a VIP program?	2026-03-06 20:13:00.483062+07
7d9d4102-2414-4ca6-994f-6331a950b02b	c487abc6-cc16-4633-b71f-032780764e3f	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	admin	reply	You need to upgrade to the 'Pro' tier by completing Advanced KYC. This involves providing a Proof of Wealth document.	2026-03-06 22:13:00.483062+07
3f58e660-d81b-45af-94ab-69cd60eb6ab4	c487abc6-cc16-4633-b71f-032780764e3f	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	user	reply	Got it, I will upload the documents tonight. Thanks!	2026-03-07 00:13:00.483062+07
3583dd66-b019-4749-80bb-0a42b4963344	58e2a2b4-a2a6-4b40-93af-90228c5fd29f	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.finance	user	initial	I sent $5,000 via bank transfer but my balance still shows zero after 48 hours. Here is my reference: POOOL-X-9982. Please help!	2026-03-06 20:13:00.484187+07
4f3018d7-894f-4127-965b-9ca0b11c68cd	58e2a2b4-a2a6-4b40-93af-90228c5fd29f	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin	admin	reply	hi	2026-03-08 20:21:11.049967+07
3b971870-632f-432d-9754-75508ab43b3d	58e2a2b4-a2a6-4b40-93af-90228c5fd29f	c87443dc-b777-47b2-a1f0-e345c92e1b47	Support Agent Sarah	admin	reply	Hello! I'm sorry to hear you're having trouble with the KYC upload. Make sure the file is under 5MB and in JPG or PNG format.	2026-03-08 20:37:38.274166+07
69a11fcf-6aaf-4b79-a32e-1bf326a2af6e	58e2a2b4-a2a6-4b40-93af-90228c5fd29f	c87443dc-b777-47b2-a1f0-e345c92e1b47	Martin Weber	user	reply	Thanks Sarah! I was trying to upload a TIFF file. I'll try with a JPG now.	2026-03-08 20:42:38.274166+07
0a0c4e28-5469-4ea1-912c-3ac1fd020ba0	58e2a2b4-a2a6-4b40-93af-90228c5fd29f	c87443dc-b777-47b2-a1f0-e345c92e1b47	Support Agent Sarah	admin	reply	Great! Let me know if that works for you. I'll keep this ticket open until we confirm it's resolved.	2026-03-08 20:47:38.274166+07
50a862f6-6147-43a4-8d56-b8c3802cf2a2	58e2a2b4-a2a6-4b40-93af-90228c5fd29f	c87443dc-b777-47b2-a1f0-e345c92e1b47	admin	admin	internal_note	t3ets	2026-03-08 22:04:34.18205+07
\.


--
-- Data for Name: support_tickets; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.support_tickets (id, user_id, subject, message, status, priority, assigned_to, created_at, updated_at, category, metadata, sla_breach_at, sla_alert_sent, csat_score, csat_feedback) FROM stdin;
21becfa7-07b1-4675-b2b9-d8a6d7e82335	0ee84d65-6576-4d63-b767-0359b06b8ec5	Cannot verify my identity	I tried uploading my passport but it keeps failing with a generic error. I've tried 3 times now.	open	normal	\N	2026-03-06 20:13:00.479341+07	2026-03-06 20:13:00.479341+07	\N	\N	\N	f	\N	\N
faacae8b-7e15-468e-a284-5528650e024c	c87443dc-b777-47b2-a1f0-e345c92e1b47	Withdrawal pending for 3 days	My withdrawal to my bank account is still pending. Usually it takes 24h. Can you please check?	in_progress	high	\N	2026-03-06 20:13:00.481516+07	2026-03-06 20:13:00.481516+07	\N	\N	\N	f	\N	\N
c487abc6-cc16-4633-b71f-032780764e3f	c87443dc-b777-47b2-a1f0-e345c92e1b47	How do I increase my investment limit?	I want to invest more than $10,000. How can I increase my limit? Is there a VIP program?	resolved	low	\N	2026-03-06 20:13:00.483062+07	2026-03-06 20:13:00.483062+07	\N	\N	\N	f	\N	\N
58e2a2b4-a2a6-4b40-93af-90228c5fd29f	c87443dc-b777-47b2-a1f0-e345c92e1b47	URGENT: Transaction not appearing	I sent $5,000 via bank transfer but my balance still shows zero after 48 hours. Here is my reference: POOOL-X-9982. Please help!	in_progress	high	\N	2026-03-06 20:13:00.484187+07	2026-03-09 13:25:28.810031+07	\N	\N	\N	f	\N	\N
\.


--
-- Data for Name: tiers; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.tiers (id, name, min_invest, max_invest, cashback_pct, badge_color, sort_order, created_at, referral_bonus) FROM stdin;
1	Intro	0	999999	1.00	#98FB96	1	2026-03-08 20:10:50.45448+07	3000
2	Plus	1000000	4999999	2.00	#027A48	2	2026-03-08 20:10:50.45448+07	3000
3	Pro	5000000	9999999	3.00	#7A5AF8	3	2026-03-08 20:10:50.45448+07	3000
4	Elite	10000000	24999999	4.00	#F79009	4	2026-03-08 20:10:50.45448+07	3000
5	Premium	25000000	\N	5.00	#0000FF	5	2026-03-08 20:10:50.45448+07	3000
\.


--
-- Data for Name: user_consents; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.user_consents (id, user_id, terms_version, accepted_at, ip_address, user_agent) FROM stdin;
92e26299-3411-4a3d-a67e-05f25888ce52	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	1.0	2026-03-08 22:12:00.170723+07	\N	python-requests/2.32.5
4bebec9b-53d0-4d16-a202-99b655c3a315	f91c696d-f3e2-4525-80e1-01a29d700062	1.0	2026-03-08 22:12:20.977692+07	\N	python-requests/2.32.5
78a53cc0-008f-41da-ab8a-77b7f776e309	db7174f5-c055-470c-ad50-b969ccfea5a4	1.0	2026-03-08 22:12:45.790128+07	\N	python-requests/2.32.5
cbc47097-f4a3-4b04-ad2f-40313baab083	c87443dc-b777-47b2-a1f0-e345c92e1b47	1.0	2026-03-09 00:10:47.434317+07	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36
a6b83a90-2bfb-4ecf-852d-c208946f9e2c	dee2990f-8070-4d34-ab17-e7fb2c041b8b	1.0	2026-03-09 01:08:00.611013+07	\N	python-requests/2.32.5
0b2d57ae-94fb-43bd-a81d-1b161fc1177e	0ee84d65-6576-4d63-b767-0359b06b8ec5	1.0	2026-03-09 10:17:12.339276+07	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36
ba80ea0e-bf1b-4dd3-9420-a32ec005d6f1	af18c46f-fd0c-4617-a959-5d7e2fb556b2	1.0	2026-03-09 16:40:20.516412+07	\N	python-requests/2.32.5
68a5b58d-cf05-47ca-ab1a-34780a21568b	edc6dda5-e21d-420d-a088-003dbcb5a827	1.0	2026-03-09 16:40:38.702967+07	\N	python-requests/2.32.5
930a49f9-7189-41b4-9cc7-f1a88874b169	3f16bd25-2aff-42df-b193-bbc3faffb12b	1.0	2026-03-09 16:41:06.068701+07	\N	python-requests/2.32.5
54ffbe41-760f-472a-b913-40e11b26704f	2e5509db-67a5-46dc-87bb-391c6727a782	1.0	2026-03-09 16:42:05.857428+07	\N	python-requests/2.32.5
1d4d3af7-62d6-4302-923d-f2e670e6d895	893aef07-c9e2-403d-9d2b-27eabcfcd78f	1.0	2026-03-09 16:42:05.892959+07	\N	python-requests/2.32.5
eb68b1d2-c029-40ca-af8d-f6e9fe9c70d7	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	1.0	2026-03-09 16:49:01.734637+07	\N	python-requests/2.32.5
4435207f-d166-4384-9274-957198f8d45f	230101db-e689-47ad-9307-076f4f0e55f1	1.0	2026-03-09 16:49:01.773113+07	\N	python-requests/2.32.5
\.


--
-- Data for Name: user_profiles; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.user_profiles (id, user_id, first_name, last_name, display_name, date_of_birth, nationality, address_line_1, address_line_2, city, state_province, postal_code, country, phone_number, tax_id, created_at, updated_at) FROM stdin;
9b42b771-6206-4f73-b6fb-79e55f0da3c0	0ee84d65-6576-4d63-b767-0359b06b8ec5	Test	User	\N	1990-05-15	\N	\N	\N	\N	\N	\N	US	+12025551234	\N	2026-03-08 20:12:16.047726+07	2026-03-09 23:14:40.751462+07
c8eff71a-1e31-4dfa-9b95-95b1fc44df1c	c87443dc-b777-47b2-a1f0-e345c92e1b47	Admin	POOOL	Admin	1985-03-10	\N	\N	\N	\N	\N	\N	DE	+49 170 0000001	\N	2026-03-08 23:28:42.045671+07	2026-03-09 14:54:18.329653+07
c2ea9fc5-79cd-4c4a-ab69-dd03f804c4c1	b45f038b-3e75-41e7-806a-0db9ca5b9272	Jonas	Freiwald	\N	1993-07-22	\N	\N	\N	\N	\N	\N	DE	+49 170 0000002	\N	2026-03-09 11:57:37.679511+07	2026-03-09 14:54:18.329653+07
7b37eaf8-1ef8-4c26-8b52-12506b1e2ec6	dee2990f-8070-4d34-ab17-e7fb2c041b8b	QA	Test	\N	1995-01-01	\N	\N	\N	\N	\N	\N	US	+1 555 000 0001	\N	2026-03-09 01:08:00.598481+07	2026-03-09 14:54:18.329653+07
a69bd946-e17a-4c39-89fd-e4800511d015	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	Invited	User	\N	1990-01-01	\N	\N	\N	\N	\N	\N	US	+1 555 000 0000	\N	2026-03-08 22:12:00.159174+07	2026-03-09 14:54:18.329653+07
8634311c-6973-402f-b7d4-603508dada46	f91c696d-f3e2-4525-80e1-01a29d700062	Invited	User	\N	1990-01-01	\N	\N	\N	\N	\N	\N	US	+1 555 000 0000	\N	2026-03-08 22:12:20.974173+07	2026-03-09 14:54:18.329653+07
9e3d42d9-b4fe-416f-a942-3558beaac63c	db7174f5-c055-470c-ad50-b969ccfea5a4	te	User	\N	1990-01-01	\N	\N	\N	\N	\N	\N	US	+1 555 000 0000	\N	2026-03-08 22:12:45.787196+07	2026-03-09 14:54:18.329653+07
89505e94-e34f-49cc-a94f-cd448c95e9db	af18c46f-fd0c-4617-a959-5d7e2fb556b2	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-03-09 16:40:20.510219+07	2026-03-09 16:40:20.510219+07
c993e046-f1c4-409d-a535-625b06335a69	edc6dda5-e21d-420d-a088-003dbcb5a827	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-03-09 16:40:38.695863+07	2026-03-09 16:40:38.695863+07
5dbdf948-7f8e-4251-88da-76dfafba04fb	3f16bd25-2aff-42df-b193-bbc3faffb12b	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-03-09 16:41:06.062913+07	2026-03-09 16:41:06.062913+07
438f6652-d66b-4cb8-92fd-a7ac9b428320	2e5509db-67a5-46dc-87bb-391c6727a782	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-03-09 16:42:05.851866+07	2026-03-09 16:42:05.851866+07
7db38052-5f19-4ac9-a396-c92689767927	893aef07-c9e2-403d-9d2b-27eabcfcd78f	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-03-09 16:42:05.889963+07	2026-03-09 16:42:05.889963+07
c8aa1730-3f42-40cd-b744-f91bfcea8ced	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-03-09 16:49:01.728884+07	2026-03-09 16:49:01.728884+07
25a9dfd0-cdfe-4681-9cf5-49abd290e435	230101db-e689-47ad-9307-076f4f0e55f1	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-03-09 16:49:01.770463+07	2026-03-09 16:49:01.770463+07
\.


--
-- Data for Name: user_roles; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.user_roles (id, user_id, role_id, is_active, granted_at, authorized_ips, access_start_time, access_end_time) FROM stdin;
ad76499e-50ce-4733-8f26-b7e4c2d31bf4	c87443dc-b777-47b2-a1f0-e345c92e1b47	3368e3a7-10ac-4e20-9c3b-015956bddca0	t	2026-03-08 20:11:28.587271+07	\N	\N	\N
2a9a5ae2-c119-46dc-88bc-1961907e975e	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-08 22:12:00.159174+07	\N	\N	\N
4212abd6-6d24-4013-a35e-14038279c547	f91c696d-f3e2-4525-80e1-01a29d700062	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-08 22:12:20.974173+07	\N	\N	\N
22ccf5be-db92-4ed0-8079-52f154386cb6	db7174f5-c055-470c-ad50-b969ccfea5a4	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-08 22:17:12.205554+07	\N	\N	\N
1b10f36c-46b5-420d-86be-6b7a11d03692	db7174f5-c055-470c-ad50-b969ccfea5a4	21c53943-2c69-407d-9f90-eed6dafd23b9	t	2026-03-08 22:17:12.205554+07	\N	\N	\N
88ac07fd-d226-45b5-a1c3-3f47ec3b1b25	db7174f5-c055-470c-ad50-b969ccfea5a4	348274de-230c-4ed5-8c5f-6ec0952d46f0	t	2026-03-08 22:17:12.205554+07	\N	\N	\N
3c3001c0-e2fe-4ed3-a520-b863aef099f5	dee2990f-8070-4d34-ab17-e7fb2c041b8b	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 01:08:00.598481+07	\N	\N	\N
8a6b1248-d4f0-4305-ab58-cee9aac287fd	b45f038b-3e75-41e7-806a-0db9ca5b9272	3368e3a7-10ac-4e20-9c3b-015956bddca0	t	2026-03-09 11:57:37.679511+07	\N	\N	\N
cf5b9bad-9234-4588-8015-657a8d3a22d2	0ee84d65-6576-4d63-b767-0359b06b8ec5	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 14:51:26.335874+07	\N	\N	\N
32a0428e-7cf0-4d5e-99a0-6abdbfb06382	0ee84d65-6576-4d63-b767-0359b06b8ec5	348274de-230c-4ed5-8c5f-6ec0952d46f0	t	2026-03-09 14:51:26.335874+07	\N	\N	\N
90fc80f8-11a5-40af-8668-c29e8e15e8fc	0ee84d65-6576-4d63-b767-0359b06b8ec5	21c53943-2c69-407d-9f90-eed6dafd23b9	t	2026-03-09 15:50:16.746717+07	\N	\N	\N
aa120fee-efef-4cf2-a066-bfe24c65ac0a	c87443dc-b777-47b2-a1f0-e345c92e1b47	21c53943-2c69-407d-9f90-eed6dafd23b9	t	2026-03-09 16:24:56.232872+07	\N	\N	\N
884fdcbb-711c-40ad-bed3-18d9593b5808	af18c46f-fd0c-4617-a959-5d7e2fb556b2	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 16:40:20.510219+07	\N	\N	\N
e244211e-9fe9-49c4-b81f-817e21d28335	edc6dda5-e21d-420d-a088-003dbcb5a827	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 16:40:38.695863+07	\N	\N	\N
58b64d78-d249-48de-9c11-cb81404f85e8	3f16bd25-2aff-42df-b193-bbc3faffb12b	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 16:41:06.062913+07	\N	\N	\N
2e3c1167-6f9b-4749-9df7-5bee4ec91afd	2e5509db-67a5-46dc-87bb-391c6727a782	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 16:42:05.851866+07	\N	\N	\N
e70015cb-6c3a-43f6-9201-f4b300a2eb21	893aef07-c9e2-403d-9d2b-27eabcfcd78f	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 16:42:05.889963+07	\N	\N	\N
cbfee3f6-235a-4a8e-8c16-8cb5d65799da	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 16:49:01.728884+07	\N	\N	\N
f2c88a86-ad2b-44d7-825e-b5c4964cc8c5	230101db-e689-47ad-9307-076f4f0e55f1	4f09bcb9-17e1-4608-bf21-f950fae5733a	t	2026-03-09 16:49:01.770463+07	\N	\N	\N
\.


--
-- Data for Name: user_sessions; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.user_sessions (id, user_id, session_token, ip_address, user_agent, remember_me, expires_at, created_at, is_2fa_verified) FROM stdin;
3a50d33e-0e71-4223-b697-be79498ac508	c87443dc-b777-47b2-a1f0-e345c92e1b47	debug-admin-token	\N	\N	f	2026-03-09 20:13:34.687245+07	2026-03-08 20:13:34.687245+07	f
1348a80c-2b8f-414c-bb85-232df7521759	c87443dc-b777-47b2-a1f0-e345c92e1b47	l299r93sPLh1kjoGtC0ZDqV-mV6uz6_d78y7DWWm-NeR8jONxLw-P14GuQo7DZffURx__VAT6uFnmffhLdoCoA	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-09 20:29:44.114287+07	2026-03-08 20:29:44.114927+07	f
9e951002-d705-42dc-ad71-4ff3e994088e	c87443dc-b777-47b2-a1f0-e345c92e1b47	LfqhcO1M7Lyp7HJtxK2w8WYbyQY_M5GFfGnDjiVbEL-IoYjx05ro-zZSuUtOkda382wIic8Ktr70vsQTJqFqrw	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-09 20:36:29.306408+07	2026-03-08 20:36:29.307144+07	f
dcd8bbd4-7c6a-472c-9b94-1977e822f92d	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	D1fRvS5gX6kDY0lH1eEti0jjZicf_skG-6Ztg-0utgBl0GGCcnUjrRvE3GYI1vYrQ0Z6USj5ZGqvdqAz9JZVWQ	\N	python-requests/2.32.5	f	2026-03-09 22:12:00.176643+07	2026-03-08 22:12:00.177007+07	f
947cab38-59a9-409e-a447-270c3cbdc52b	f91c696d-f3e2-4525-80e1-01a29d700062	H64auaAr5842vkyXQV8P54MvxYsa7V-FeA8eTooUdQRozypiLlVxx0zKREKGtCa6pzrpyu7i9dXQhMn9KECHLw	\N	python-requests/2.32.5	f	2026-03-09 22:12:20.97926+07	2026-03-08 22:12:20.979439+07	f
2b26272a-9919-452c-896a-7445a4bcb04d	0ee84d65-6576-4d63-b767-0359b06b8ec5	test-session-token	\N	\N	f	2026-03-09 22:12:45.444143+07	2026-03-08 22:12:45.444143+07	f
6a32dc6e-7fb0-4e03-ab08-fdc19157ea35	db7174f5-c055-470c-ad50-b969ccfea5a4	MfgKvfGGFh_PkufyOAZ8v5fP333_Zfnt5SdiYwxHD-REzcF1kz88XSIHLbSze4jUWSj5v0e0drO56wK9FCOQHA	\N	python-requests/2.32.5	f	2026-03-09 22:12:45.791088+07	2026-03-08 22:12:45.791199+07	f
133c4a97-5555-4f3e-a2c4-8df3fe13077b	dee2990f-8070-4d34-ab17-e7fb2c041b8b	rhwq-hdm2YslQm4m8vj8itwVXo6glK1c4SRuX1mF-pnX2rGo5TI5M0iSU37a7N-hEE6lC88EXBVpdoeF-CE_Cw	\N	python-requests/2.32.5	f	2026-03-10 01:08:00.612173+07	2026-03-09 01:08:00.6126+07	f
40dde3c4-c7df-418c-941e-efe7e3c172d0	dee2990f-8070-4d34-ab17-e7fb2c041b8b	WpF0xMPWZAR0JtX9l9Ew-4YXKAe_U4DLMD0LY5ltf_q4pCKChnyuVBgjXahqFNQqE6s9eQZN1aicNY5Tal1v-A	\N	python-requests/2.32.5	f	2026-03-10 01:08:00.853665+07	2026-03-09 01:08:00.854126+07	t
5da5085c-df72-4927-aeb2-ae0d49a83d5a	0ee84d65-6576-4d63-b767-0359b06b8ec5	eGUr679EM9X5zZ3kFIclDzOHf---RHk9pIgpPGNYQXbvMSGX025xNDAymh12boOKUtL0veEJNrLzlu4BNSQ2zA	\N	python-requests/2.32.5	f	2026-03-10 01:29:12.575744+07	2026-03-09 01:29:12.576021+07	t
55892398-2f93-4fd1-8da6-498149835b1c	0ee84d65-6576-4d63-b767-0359b06b8ec5	Mk_C4CokksZ_djBd4yMe2X_eT_Ha-8si_s3oW3ZqO-iZfqzYsnAfQhT-etMJnocn4jvqiQcJeAq6UEpKar6woA	\N	python-requests/2.32.5	f	2026-03-10 01:29:47.923392+07	2026-03-09 01:29:47.923784+07	t
8a065012-4a5a-4148-9e30-06de20d0b9f4	0ee84d65-6576-4d63-b767-0359b06b8ec5	C0QpgWzc0GcDnsSrlMU3lgC9qYe7YKKN5CYwvc5k6KbADo0DbgIGA4uXGY1kwJDXQ0uOca1tVOf9Ktg7vF789w	\N	python-requests/2.32.5	f	2026-03-10 01:29:53.948726+07	2026-03-09 01:29:53.948845+07	t
2828a17c-2599-4a94-b7c7-8673eda596f9	0ee84d65-6576-4d63-b767-0359b06b8ec5	RReoQ65Al5-TdTvCY93F3prSxIDL05Txj9pLhcgxmXxmJgMUB4jCfdrIvUV-S3GNisrEzSHiollBTILfHlDHaw	\N	python-requests/2.32.5	f	2026-03-10 01:30:00.83652+07	2026-03-09 01:30:00.836722+07	t
a6dbb23c-bc0b-4f94-9309-e7fdc52eed53	0ee84d65-6576-4d63-b767-0359b06b8ec5	IY0vkGnqr7vFE2JaMbdN4V2gDRt94xGm575uywl0EDdtEb2crXdnllrI5QGkOc3Le6zY9-brmX8Ot963Vg9y4A	\N	python-requests/2.32.5	f	2026-03-10 01:31:19.844377+07	2026-03-09 01:31:19.845096+07	t
7c5a61ae-de92-4c92-9a12-66821f3ea075	0ee84d65-6576-4d63-b767-0359b06b8ec5	KYNF8Qdxc4OTlQTDs06OM8Ps0RL4zY2dv0fhMjcYAWvKF3QBvpAixmApFL9c9qnzMS9DDvFtGqZuCvvH4cm3Eg	\N	python-requests/2.32.5	f	2026-03-10 01:41:03.205639+07	2026-03-09 01:41:03.206588+07	t
f8715a1b-af81-47a3-8ef4-8f4238fb61d9	0ee84d65-6576-4d63-b767-0359b06b8ec5	EOYRvaXln2n8nj8OBy3vBKcedeSazMf0YTGfKpXezFog369RgHMzoOHG6XOlBohO1JN3LTJU2DkTdwUpdLBLqA	\N	python-requests/2.32.5	f	2026-03-10 01:43:29.346211+07	2026-03-09 01:43:29.346814+07	t
b7062ec2-98ad-4992-a5f7-bdfeb424acb9	0ee84d65-6576-4d63-b767-0359b06b8ec5	NLNQKRRq0DVXEimMg3tXI7Qjl2XMC8PAWmqqMl63nZXvroAsR4x_jCfwHnLwV3qyjrXKYU_A6e6Gr5JRLShKWw	\N	python-requests/2.32.5	f	2026-03-10 01:45:03.235004+07	2026-03-09 01:45:03.235519+07	f
bd7183fe-f8f8-4870-bac6-b7e779a10e58	0ee84d65-6576-4d63-b767-0359b06b8ec5	ZVs1OyW7V88AmGCwkUkTBnNthxbOW1i2Wwx7mO_2zOPoMaorqaPt0r7Vi4OMjf9ov4j9w5dGm_Q__rQXiO4jIA	\N	python-requests/2.32.5	f	2026-03-10 01:45:28.406535+07	2026-03-09 01:45:28.406824+07	f
7e82a245-d4ea-465b-974b-d013e50f8e76	0ee84d65-6576-4d63-b767-0359b06b8ec5	PhqevWEf4VbM4yvgXj2Ut4ERCbzNd882TgA3mfX-dn-kW0IGIrypBti9xe2n15Ibz73Il4qP4RzVcFaSO_HEQA	\N	python-requests/2.32.5	f	2026-03-10 01:45:34.750183+07	2026-03-09 01:45:34.750293+07	f
078b73e9-2378-4636-a9e5-d3ce46d6ba6a	0ee84d65-6576-4d63-b767-0359b06b8ec5	EWwnaA-OAyZUlPraSqej5f8AxIRW5KROiESnzSiq660q-Rv_oU5clMHFEeVbG0koyCa0nF3XYMQ11yig-WHOXQ	\N	python-requests/2.32.5	f	2026-03-10 01:45:57.705514+07	2026-03-09 01:45:57.705709+07	f
af518361-d87f-4e67-b719-6bb069e0ef17	0ee84d65-6576-4d63-b767-0359b06b8ec5	r7-_KJ1hDTln9u5mqrye42HLxwm4GiAheryWARybSRnayU271Wv4lhGgXgktmDW19R-dUjmkdB3SwtJxybAYrw	\N	python-requests/2.32.5	f	2026-03-10 01:46:02.314537+07	2026-03-09 01:46:02.314705+07	f
31879210-ffce-4614-a051-6c2323bc6391	0ee84d65-6576-4d63-b767-0359b06b8ec5	QnoGXsf1CERnxNpAcKrYW-HWwpZ1B5_XS_w-ozfStFIVQUwxfUpfxocDhpMTyWQggY1CcJ1O5fEl7sSxVLf7WQ	\N	python-requests/2.32.5	f	2026-03-10 01:46:48.946315+07	2026-03-09 01:46:48.947068+07	f
6ab8b3f4-7641-4b49-b9bd-a2b3c4eacd6a	0ee84d65-6576-4d63-b767-0359b06b8ec5	P4Iyj6aBeD51QnP_aTAn0qT5pxcT4zk7ZHsvE8tYl7Q5wtgny_tYG3CyiP7qn_EKwb56zRZYf-7cBWQFOeS4ig	\N	python-requests/2.32.5	f	2026-03-10 01:47:25.94869+07	2026-03-09 01:47:25.949258+07	f
09358d2c-ca41-4292-b087-3cf85018e670	0ee84d65-6576-4d63-b767-0359b06b8ec5	sR1YhMuhra_nVYgT7GW12aNW4ShunboYaqOuC1BYN2m_CL18P8jH6UGS4X6vyTTXxxTxPsMXsVPSq-gwaQVmkw	\N	python-requests/2.32.5	f	2026-03-10 01:48:02.806107+07	2026-03-09 01:48:02.806274+07	f
b0da7fed-980b-4313-ab9b-714bb328f113	0ee84d65-6576-4d63-b767-0359b06b8ec5	K_qiTrYC3Fg_Hhnl2zK2IgvVd4ljdryM_XKobRs6GTdtGhMmauhNvcMsfqROabcIS82MP0qbET2828TunRj9sw	\N	python-requests/2.32.5	f	2026-03-10 01:48:28.154736+07	2026-03-09 01:48:28.15491+07	f
0ba29357-36a7-4d07-b52f-5596d379df1c	0ee84d65-6576-4d63-b767-0359b06b8ec5	HVR6TXB1DEitGaXhok9pwsF_AZxKIrE1-t7sbG1YwSvYZVUDVgi2h1JXNnGHdAn3fwn32HgAAltDu6lnctVHZw	\N	python-requests/2.32.5	f	2026-03-10 01:48:32.975354+07	2026-03-09 01:48:32.975446+07	f
eb0e570d-fda0-4cf8-909f-0d94eaaa1bc7	0ee84d65-6576-4d63-b767-0359b06b8ec5	9q5DwKqeJoSDz9nr0wwyZXWdUAm8RZFfxmc6T1xpYBmcAi5-UsButPjWh5RvUTinwKP4xIqRnaTYnJ8wJ3AMaw	\N	python-requests/2.32.5	f	2026-03-10 01:49:02.899939+07	2026-03-09 01:49:02.900067+07	f
6069397b-773c-422f-984e-cc2387634105	0ee84d65-6576-4d63-b767-0359b06b8ec5	x4wmJw6qSFeZFNnmcoxEUt2jrj9TOWg2fzQITMMMzA5Vk_TglxOtq66kgBlt1PY2BGKxvGcQNfOFlNLRHAWSPQ	\N	python-requests/2.32.5	f	2026-03-10 01:52:11.956106+07	2026-03-09 01:52:11.956604+07	f
424c6eee-1aca-41ff-bdb3-e4806f0889b3	0ee84d65-6576-4d63-b767-0359b06b8ec5	EECmlGltendg-8GtH3XYx6Si4PSj_rgOiNFt7FnrXzZD4ZvVMBNsl7e6RVhYjkM9IL0hVP4O9e0BU9PugDqBPA	\N	python-requests/2.32.5	f	2026-03-10 01:52:46.620515+07	2026-03-09 01:52:46.62065+07	f
bb3f2f2c-c6da-4283-95fb-ac959b65ecb2	0ee84d65-6576-4d63-b767-0359b06b8ec5	E3hVk2Dc-NBYDmk9U_g9wrtZNPxBhrQoNg0r76RD2j1PSyQfRzwaIoRWzcRLSBW82zPbLLwdDAGIdZFHr2_lwQ	\N	python-requests/2.32.5	f	2026-03-10 01:53:10.482914+07	2026-03-09 01:53:10.483124+07	f
a8793361-5afb-4635-a557-e737d03097dd	0ee84d65-6576-4d63-b767-0359b06b8ec5	Zm2NnLiaJ_YHW8kP5nJOMpEPi0WF6E_vTijU93tmJuWR0V0bwXbJSErAjtitLdu6tNeKuuRx7aCIuZVM-DvM2w	\N	python-requests/2.32.5	f	2026-03-10 01:53:50.962146+07	2026-03-09 01:53:50.962266+07	f
ff56b4de-860d-480e-9abc-43baf0d527a1	0ee84d65-6576-4d63-b767-0359b06b8ec5	_2-GwuCV3-hgO0c3x4ULQHyGkpWWG0N4WjEKEvoYVZ1r4Ymtxy1TQGjjXI9PA34TieawkquDjs1v1u4vqI3PUA	\N	python-requests/2.32.5	f	2026-03-10 01:58:53.352785+07	2026-03-09 01:58:53.353258+07	f
b55c31ee-0741-4723-bc23-957891ea608f	0ee84d65-6576-4d63-b767-0359b06b8ec5	ttjHK4af6dlyywu8H2BAN6p_HZtra3bgWQUZyOBXOUxJGvqQQ9QGUdH4HZaVgPkDRFpIEkjwx7LLD1Q8Oym2nw	\N	python-requests/2.32.5	f	2026-03-10 01:59:02.730233+07	2026-03-09 01:59:02.730612+07	f
273e12e8-14c5-4615-b124-8da5c3d58ec6	0ee84d65-6576-4d63-b767-0359b06b8ec5	zR9h8IHDgEYVmmH-qxVT2knb0dcPLytjXstQKIJcpDu2H9rVmj9Ii12ugs_AXnV2V1Pvh6kVuXJXCRlWjm2xGw	\N	python-requests/2.32.5	f	2026-03-10 01:59:04.749182+07	2026-03-09 01:59:04.749364+07	f
043d3e5f-4625-4abc-88e6-86a475ee457d	0ee84d65-6576-4d63-b767-0359b06b8ec5	HS9jESjvkhQkPQS2Z9NrFa7HnHlOYy_YXxe3vQfmtexFc36JQPX7M6u_C-KuMIEcyUGrKWaFgCqY56PoVGF5Zw	\N	python-requests/2.32.5	f	2026-03-10 02:02:46.104919+07	2026-03-09 02:02:46.105344+07	f
f5d54815-7c5f-48d8-88a6-a676c730e7c0	0ee84d65-6576-4d63-b767-0359b06b8ec5	td-xo0QDClJAYTPgj-EG6rySQRWyJaWfK_hmDEFrznHoO0VcMyb6srcUw7JNDSR7ijUl8d8h0lNy9CPODS58Ow	\N	python-requests/2.32.5	f	2026-03-10 02:02:56.376659+07	2026-03-09 02:02:56.377113+07	f
a1945483-daf0-4c27-9e65-652302780532	0ee84d65-6576-4d63-b767-0359b06b8ec5	slB1F5MxXtFQv1ibeWs0C5_dkIEAlwQfs-Y3t55k1M658degSOLJfjYgYhzfXePB3TggfNl3-Y9zNL6A_MP76g	\N	python-requests/2.32.5	f	2026-03-10 02:03:10.83168+07	2026-03-09 02:03:10.831844+07	f
8757acf4-5bb4-4a4e-8c13-9dda49840e4b	0ee84d65-6576-4d63-b767-0359b06b8ec5	FmZkk9ul7vMaeTWp8QMsxc1cTm8u_bmJ8kPVR9S8zuWiEsJhbud7rj0cWqwCUfO6E5cBaGdjyoaFvcYkbhTDpw	\N	python-requests/2.32.5	f	2026-03-10 02:04:19.623622+07	2026-03-09 02:04:19.624117+07	f
13df6bb1-6755-42f4-b4e3-c7893a29e34b	0ee84d65-6576-4d63-b767-0359b06b8ec5	rKGxmP0E18o_dAc8BHPJJFwxctmsCQdOvefN3764ztLqqBGYn7B8z5ro_OqEh0aoQJHeBC5bS56GB_7xM84A6Q	\N	python-requests/2.32.5	f	2026-03-10 02:04:20.813832+07	2026-03-09 02:04:20.814082+07	f
4b7e32d8-7323-4f51-aed5-653fe197b404	0ee84d65-6576-4d63-b767-0359b06b8ec5	Xwgzp5rAEkOf6GUA77DwbyJA_wP_IUhdUm6ixsXgNOr1uMNr_TrcKzYlcX36LiEMYgFwbbufk6FnxmOWYiMEuQ	\N	python-requests/2.32.5	f	2026-03-10 02:08:38.638967+07	2026-03-09 02:08:38.64008+07	f
c8d14494-b4d6-40d6-b5ab-25341ea72ae0	c87443dc-b777-47b2-a1f0-e345c92e1b47	roq2NfsyXJxGPZz67PfRQ-TVPF_wEbRfSIGpVScBhaYz_buvmf3WVWJcAgGX3ERI6hfwojIcBUklaX7MGF7WtA	\N	python-requests/2.32.5	f	2026-03-10 02:08:42.309101+07	2026-03-09 02:08:42.30925+07	t
1d391bf6-646c-4ac5-9e9f-62d13de0b546	c87443dc-b777-47b2-a1f0-e345c92e1b47	2F-XMHKh3h1-_xn_nvmeX5T9OdeSn9bQXG4XnstBBUdgPjn21px8YHv-r-3ZcgEWa4HAZDhYuz0evQutXWrcCw	\N	python-requests/2.32.5	f	2026-03-10 02:09:31.861419+07	2026-03-09 02:09:31.861815+07	t
096e797b-5ea5-4f8b-b866-cc6c6a332f5c	0ee84d65-6576-4d63-b767-0359b06b8ec5	XLAjTsHDVK5VkwLl6qFF2r246e_JmG0rF0hjzWjUwd9xjNo5cx0O_P7qxq5Egxb40qhddJHZRsby-QlETIN6OA	\N	python-requests/2.32.5	f	2026-03-10 02:09:32.155832+07	2026-03-09 02:09:32.156201+07	f
68cf12db-615e-4500-8744-5058a3d69378	0ee84d65-6576-4d63-b767-0359b06b8ec5	0NROODhV-XOFjmAoWzj-rbfMMnjNJWpsBv1Rs6zFsS9dayo2YkH_BG2gGD91g4UPk4Rqk7N_8DpUKYvJP07ouQ	\N	python-requests/2.32.5	f	2026-03-10 02:09:34.795185+07	2026-03-09 02:09:34.795302+07	f
b0488a02-ca40-4245-bd0a-02796f3eb2a3	0ee84d65-6576-4d63-b767-0359b06b8ec5	fAHkrS5fnvm74bXWceK50cbJs3lnH9wH_MDTuwCkd7lY_jvPNhP3dpp9IbKg-6-M2miunHXyjryIuasrF-6AIg	\N	python-requests/2.32.5	f	2026-03-10 02:09:41.197147+07	2026-03-09 02:09:41.197334+07	f
3aa4fcab-df2a-48c1-a339-b85aa9f75304	0ee84d65-6576-4d63-b767-0359b06b8ec5	unAYM49aRI2iGQ8x4M5gUET5tOj7lcSCSSihJDxaRI41iVk6IPewztxObUw6LSFREvlXZ7-NT4dPdov7JbH2Eg	\N	python-requests/2.32.5	f	2026-03-10 02:09:59.640843+07	2026-03-09 02:09:59.64136+07	f
01007fee-9e6d-4fc9-a9b9-b52297970b49	0ee84d65-6576-4d63-b767-0359b06b8ec5	RD0vv3Yp7q2fcc6x2dm6qnFhSuFhkZB3VPrCNxtwc0rqT8fF-6Eyhhy0iRhDvrSCtYevfQbN_XaN4Vk_CV3NEA	\N	python-requests/2.32.5	f	2026-03-10 02:10:07.857572+07	2026-03-09 02:10:07.857739+07	f
dfcfca49-3734-40a2-830d-26353e45297a	c87443dc-b777-47b2-a1f0-e345c92e1b47	NzRuj5rhTebiyQBoGIG1w9ubMq8oQgnEcPUzxKH9B_eMgzOodRbF1RPE1Q-X2B6yVs3ErSPSA_Eq0wuKIWnNgQ	\N	python-requests/2.32.5	f	2026-03-10 02:10:08.673848+07	2026-03-09 02:10:08.674097+07	f
6220a0ff-2b51-4352-beaa-17892fae0638	0ee84d65-6576-4d63-b767-0359b06b8ec5	1TDu6vlHtO_gwbo8dgxHREVRgtIlKxg8CMj6YqwdFnD8l4yQug52y0i-iPCMAJK_AljtkQFDB7D6NeiQLe9ZrQ	\N	python-requests/2.32.5	f	2026-03-10 02:10:32.321797+07	2026-03-09 02:10:32.322288+07	f
5f908ddf-74cf-43b0-9e51-b4f7bf6669e3	c87443dc-b777-47b2-a1f0-e345c92e1b47	8Lw603KJBaqa57skVDP4mNda_SleJTjTIeQ5chCI9uugRLfWg17Ej6hOtTp2iife9aE3hq5QXaG3MlJuu1WU6g	\N	python-requests/2.32.5	f	2026-03-10 02:12:46.98773+07	2026-03-09 02:12:46.988105+07	f
798a32c1-f3fc-430b-9051-0a3c7c337eaf	0ee84d65-6576-4d63-b767-0359b06b8ec5	R6noRt2U11MOI2guseVi57evfaWIScWtolgxoXYMHHzP6ejLaN-ot0sjjjNCJIz7aeTX7wMwhUzR3I4S1TMQWw	\N	python-requests/2.32.5	f	2026-03-10 02:12:49.061618+07	2026-03-09 02:12:49.061994+07	f
7ad8b1c2-5e74-4190-8c40-07855858eb1c	c87443dc-b777-47b2-a1f0-e345c92e1b47	j1YAuOBC2mIbgQeEdDiJp6CGnQWmzslaYQbYr9CeYiADjKUTbnvRv2KvVWr_UIqn_iK-nrUg30QcDkeccvJKhA	\N	python-requests/2.32.5	f	2026-03-10 02:12:52.866818+07	2026-03-09 02:12:52.867201+07	f
39fbdef1-5ba5-4b13-963e-2cdf128cc646	0ee84d65-6576-4d63-b767-0359b06b8ec5	f0PdFFzhbvHtb0MZJeiqlylJ3-LzYHVbWYu4-zhVn9BZuBq-nKt8djzs2lVCwLdkMcgzI4bMCliImnpVZ2DDTw	\N	python-requests/2.32.5	f	2026-03-10 02:13:03.868672+07	2026-03-09 02:13:03.869119+07	f
c449d3ce-ae40-4f99-a9cd-104371690483	c87443dc-b777-47b2-a1f0-e345c92e1b47	TXxxXrzqs33t9jiW4zfRHoqg_mKsOYDQe_xHvRoAbahH97ZBoZMw6Dyv8ESxw-EMzbasz5IvrnFyRgANEvkkxw	\N	python-requests/2.32.5	f	2026-03-10 02:13:28.830712+07	2026-03-09 02:13:28.830836+07	f
cbdf598b-50ef-4415-bd0e-d59a56d85ee9	c87443dc-b777-47b2-a1f0-e345c92e1b47	GqmoPyerF7X-8G1zNrab_yF1b4xDCfJ8MSNlMRustEF6Tbt4cigswAAzSpnd1_eNSVYNCvUpN0N5DJFicSf3_A	\N	python-requests/2.32.5	f	2026-03-10 10:12:25.786774+07	2026-03-09 10:12:25.787212+07	f
73f65084-5e2c-4e2a-a632-2c778297e092	0ee84d65-6576-4d63-b767-0359b06b8ec5	50ZU4lQAUxmkDKisMbg1fQ_7Znx8fyjzL2oP_ZbwaTJ4iyJmh04gEGbg-t2T3HiNp0lUdZ8jNbuFc2QtwLTiKQ	\N	python-requests/2.32.5	f	2026-03-10 10:12:26.477526+07	2026-03-09 10:12:26.477929+07	f
86702b6f-969f-4253-939e-f083408813cf	c87443dc-b777-47b2-a1f0-e345c92e1b47	cfbhRX9qgcae8XgI9m2y4h0LLKGCeVLJ-PfZQ3fig7jwSeRgQQFnZRvCKiUUUlBKN3wuko-yvACq0GO3je1jIQ	\N	python-requests/2.32.5	f	2026-03-10 10:13:11.863171+07	2026-03-09 10:13:11.863325+07	f
0daf7d60-4945-4cef-84c7-12c4646494ee	c87443dc-b777-47b2-a1f0-e345c92e1b47	azuy8o6PjtrUADPC7xSrKzN9CdwkbCSY3YshymJVUL76rKteWeMb_R6C1nG5VmuCUQsCfxH_qtoxgbWA7EaFrw	\N	python-requests/2.32.5	f	2026-03-10 10:13:24.195995+07	2026-03-09 10:13:24.196162+07	f
3e2a4102-28b4-4e76-9224-0ec60a21c062	c87443dc-b777-47b2-a1f0-e345c92e1b47	Y3z5hbpe6SbINQ1_xrxn18Kuos9pfd4WuJVEfOuH3b4SYoIZHCWr41O8zOv2s3Ou4HUVbdKTlc2p0hqmezMe4w	\N	python-requests/2.32.5	f	2026-03-10 10:14:00.429659+07	2026-03-09 10:14:00.430348+07	f
ad4d725e-df14-4cb9-bcab-14c290622495	c87443dc-b777-47b2-a1f0-e345c92e1b47	i-Kl5VKlWMYKERSluSz7-WIQ5BggnIGg4IIjMFj0L6Ii-hXnySTG3gVt-0_QI1DPyT5MrygmzoVnhtP0SGm-eQ	\N	python-requests/2.32.5	f	2026-03-10 10:14:06.377273+07	2026-03-09 10:14:06.377744+07	f
3d7900a4-1c91-496a-8e46-8e2258f141be	0ee84d65-6576-4d63-b767-0359b06b8ec5	x7fhrqbVPg2BhIBQJNAHJC88uISN0gXaslXd4hfsCShT1lHodoUas7_Znf2mgVTDxswQgFWLK22RGrQVld8Hfw	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 10:14:47.383698+07	2026-03-09 10:14:47.384171+07	f
37bc18f6-dc40-45cf-b13b-e20f9138e2a0	c87443dc-b777-47b2-a1f0-e345c92e1b47	4atg5EgRp8811clOrjglOZAaDJ1p6wU7iFk-EQTp5aVc91GQzd597kW20JvaiBV6pnd3uk7nPPQvn3VtV9Pr3Q	\N	python-requests/2.32.5	f	2026-03-10 10:14:53.407317+07	2026-03-09 10:14:53.407528+07	f
d823e7b8-3a03-440e-9caa-24648797d235	c87443dc-b777-47b2-a1f0-e345c92e1b47	w3mtUifMJ34ymUNdrBNJMzdOOTPulGdUT4HWtEKopPzmvJgSY8lnXxH1XYbhf7kZbby5GcPl0VvehujiaFOAVg	\N	python-requests/2.32.5	f	2026-03-10 10:15:13.802665+07	2026-03-09 10:15:14.047636+07	f
9d4775cd-01ca-4a3a-8482-a683b6790884	c87443dc-b777-47b2-a1f0-e345c92e1b47	K5Ge4oZXJGuYBfsH1JcqTWOqmcyh70kxH6VNCToDIK9Hw-QhyvG_OW5mi7Si6914DxFJaNkzTbyij6pp_42qTg	\N	python-requests/2.32.5	f	2026-03-10 10:16:08.459429+07	2026-03-09 10:16:08.459601+07	f
4af48830-6c6a-473f-9396-00f86e241d6b	0ee84d65-6576-4d63-b767-0359b06b8ec5	8awUoWyXdBaszNcYQzv_u-MS1KFVMf8sWiOpKCorVZ8FlkxC_utL36CoKSQX6o3afT4AZGq4hk2VNKwb9ZBZ1g	\N	python-requests/2.32.5	f	2026-03-10 10:16:18.145602+07	2026-03-09 10:16:18.145765+07	f
dafd17be-24c1-40da-abbe-f7bddec29385	0ee84d65-6576-4d63-b767-0359b06b8ec5	N5UtXiFPCh7RqMO1VUX9UdPCVUPkOUKVuR_x__B1ePGiJbw9D4ewaFuKPVkz-JP67Hse03X8lC8RAtVSEwyLpA	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 10:18:52.528497+07	2026-03-09 10:18:52.528963+07	f
08b7f15b-e8e1-4a11-a0b1-46bd32dda792	0ee84d65-6576-4d63-b767-0359b06b8ec5	buDWcmOtxymBtG7w8D2JTbbi9ZWo17gnWemEUF3UI_Gtr-MLkYpD2mT5dWaN9dP9afx57ThyFfT5JdRZ2ss9sw	\N	python-requests/2.32.5	f	2026-03-10 10:32:02.492899+07	2026-03-09 10:32:02.493326+07	f
9f0b52c2-3751-4662-ac5a-57dc6f0bdff1	0ee84d65-6576-4d63-b767-0359b06b8ec5	sThapa-MGtitkLHUFuotV1tTInHYa5kiXC2ZvLaacSZsunCjZBoEqhkV95tQPbBFYXQYBcBhcUAqG9hpc0YY9w	\N	python-requests/2.32.5	f	2026-03-10 10:32:48.624725+07	2026-03-09 10:32:48.62494+07	f
7913507e-e2c5-4f68-aa78-7f67b0eb60df	0ee84d65-6576-4d63-b767-0359b06b8ec5	LzhAXmbWIugqPclEw5te6DtIi0P24R6sPYujRpOXt-vfA0A0ULIOps_goY7SnkLxSb0OW-KvtEKywJaXeKIWOA	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 10:33:19.58457+07	2026-03-09 10:33:19.584743+07	f
5d12f16f-b931-41c3-8eec-f5f7fe9969f0	0ee84d65-6576-4d63-b767-0359b06b8ec5	1JovAUWPLDdqey_viAAkfK7klFdwN5kh3fl15gTz1d9nJ268hgHBF0fXteJ2rKziEZOelySNEA9g-ebOy5R_Pw	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 10:34:06.179569+07	2026-03-09 10:34:06.179681+07	f
f7fcbc4f-6609-4241-8ddf-9d1a7be7a13a	0ee84d65-6576-4d63-b767-0359b06b8ec5	7WRJpAvgAQqnV7hoF1vBgv0uYdJtARGjF7j2NVcIDpjyyvvb15haze6FNCWgY2f2mNnHA_jQPt9hvCnWqEK3Pw	\N	python-requests/2.32.5	f	2026-03-10 10:35:51.660268+07	2026-03-09 10:35:51.660982+07	f
384dc209-c06c-4d87-87ef-e6615c6031b1	0ee84d65-6576-4d63-b767-0359b06b8ec5	9NoUkI2AyLoaJi7uX77Ez0rMCcg1_wx53nysBDE2Dn3zirEzTgsAzbihKhYnYIEyAN5I4PbAwm-zZTQD9WgUXQ	\N	python-requests/2.32.5	f	2026-03-10 10:35:52.723889+07	2026-03-09 10:35:52.724077+07	f
1a5f037b-70c2-4ebf-9cd6-8f57360f73cc	0ee84d65-6576-4d63-b767-0359b06b8ec5	-bgOVNc9gk_CcZOocCN0E2c7ZdGsgJVeTeqJACWs2r-0vWxcxE7GHnJtEGPgM8OpWBjMEQDgIhZxWBqBS0asEA	\N	python-requests/2.32.5	f	2026-03-10 10:35:58.419713+07	2026-03-09 10:35:58.419837+07	f
81ea49f1-1f32-4745-b73c-d4ed2f6029db	0ee84d65-6576-4d63-b767-0359b06b8ec5	iZ8uzy6TqGn9D1xaWs2NjzzDOzsTfOfRokMUr9Q_SuZfwi6bOmWKtUzc-ZcgbFSYCHs6Jd9zzHd1s6sTIz-evw	\N	python-requests/2.32.5	f	2026-03-10 10:37:37.324548+07	2026-03-09 10:37:37.325043+07	f
2f5db579-fe1b-47e9-aea4-f8fbc4158771	0ee84d65-6576-4d63-b767-0359b06b8ec5	dUswPLpXpu4sVslFgKVY7m0o78rGKsY_SVTklJ5xh6XkPtrw3-odc8ZLl8RVimagjccrRBpr-fT53B6c7-Hjyw	\N	python-requests/2.32.5	f	2026-03-10 10:37:53.083479+07	2026-03-09 10:37:53.083584+07	f
e2995363-ccc0-4509-b8d5-20f6de8cd890	0ee84d65-6576-4d63-b767-0359b06b8ec5	wLyD2gt9CvLNLj8cGZleTTTnzIHA7srCwW9Pxpk4DKa8LxJXuTtvsKpqitEhjjQ39B9BHExJxEf4IfxQYmgBXw	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 10:53:03.612994+07	2026-03-09 10:53:03.613368+07	f
3f82ca50-ad14-4470-9a36-0dfd1316eb4c	0ee84d65-6576-4d63-b767-0359b06b8ec5	9kVhhGXs3TnJvPZ8Omk8RWVEbxV1xoWXHOvqT-z_ihe4yJdDIKagx20ZunsuGTCg-58W-aZZE06RZBX0n9091g	\N	python-requests/2.32.5	f	2026-03-10 11:06:23.766235+07	2026-03-09 11:06:23.76841+07	f
8d0ebde7-29a3-47cd-88e1-34d2e1d1a3bc	dee2990f-8070-4d34-ab17-e7fb2c041b8b	4-3vHSq-xDHEDTEEqkrXBazH16zyVUehBqPIq1uWRk4jt72jlXGP8p1BcKYWDdw88l50qx88OgaKEQtT_l5S8Q	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 11:08:51.856508+07	2026-03-09 11:08:51.856836+07	t
ddbac73f-465c-4ad6-9e6f-1f7ff7f42683	0ee84d65-6576-4d63-b767-0359b06b8ec5	ZjnnD3kH1rR9OaPYb4u8cCmyhhJ527eNzPRYH3gaT6Viy83FSi1u7aIIkdupbtV9wqwCglJwngFuev5Gktl7FA	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 11:11:19.072973+07	2026-03-09 11:11:19.073096+07	f
ad49def3-d317-426d-89cd-f9ee7aa9a90a	0ee84d65-6576-4d63-b767-0359b06b8ec5	jTdvW6QCYE1qP-I45UE7DJOGWl88n4TEWroEROEX8XtTZvlEzRPWm5ObCujDmG_a8eUqKP-D1AAM3-wy_2fNKw	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 11:10:26.303726+07	2026-03-09 11:10:26.303877+07	f
0eb35b35-fe78-47d8-b7fc-71ab51dd2b21	0ee84d65-6576-4d63-b767-0359b06b8ec5	pka5vsgeWn4rvanFEI6hCHpi2wxIiz7S9z172ky-n70mG8ZQAjZwkap0L_80Bns5vhbUpFanQeyHpXE4qHgl9w	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 11:11:36.471273+07	2026-03-09 11:11:36.471563+07	f
72e1a88f-343b-45b6-8114-107b536d4f8d	0ee84d65-6576-4d63-b767-0359b06b8ec5	88KKXITWa9sC_F0S0hme9kxlCm_kiHfFbxX-oyrIQPGH_NLRPRNRh4W3ltVMGHgEgcmJQFMORMIlloEbv04HrA	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 11:12:05.767637+07	2026-03-09 11:12:05.767977+07	f
02b868f4-4710-443e-a483-2dcfb52eab25	c87443dc-b777-47b2-a1f0-e345c92e1b47	LYm92LVuoZxgVfnIaxeuWEElZ0y6V2PdsKEoQd0Tj36mMarFltiA47bU6fe7BhFzxm5MCqUAbT7BEPWvRRNLGA	\N	curl/8.7.1	f	2026-03-10 11:28:53.156022+07	2026-03-09 11:28:53.156655+07	f
d69e2dab-e10d-45ee-b9da-ed61fead9f1f	c87443dc-b777-47b2-a1f0-e345c92e1b47	dN7aMVdYNR_eYNHwxwSihf6Tmv1GvmUBQ67mt_cE75G8jlRMF8cCmRtjPg6s5zNugdRpNF3lqDypO2DKwQhS4Q	113.23.36.123	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 11:50:04.372267+07	2026-03-09 11:50:04.37287+07	t
03cb76c2-6415-4e3a-8f70-50eb54b013dd	0ee84d65-6576-4d63-b767-0359b06b8ec5	e8XZNzSiSzh5aZPPagbovJt8DOdO2qnNUnFmZZtX6PCmyhmbFKSA38EKX2UVXpwEIELGVxDgDw6RF4Zx0_ovDg	\N	curl/8.7.1	f	2026-03-10 11:58:01.165258+07	2026-03-09 11:58:01.165733+07	f
a087f77d-3993-4c4e-9074-9abc2d49823b	3f16bd25-2aff-42df-b193-bbc3faffb12b	Bc12cLOG8RUS3tSALR0ZJMt4Nc_57v-Ug-iHtMHYhzRcy3Po8tGG0AlClXFUzd5nGBloTmkf8QbrjBoVnjUMAQ	\N	python-requests/2.32.5	f	2026-03-10 16:41:06.069446+07	2026-03-09 16:41:06.069846+07	f
061aa6fd-eb0e-47b5-aee3-3a7e67c31748	c87443dc-b777-47b2-a1f0-e345c92e1b47	_7175JHdxfd14FMZeAqwPNf8mwguI6SwOo4JfAJIHVRIR2BJ6UwAUglTVAmxmAMjFqd83DG0VuLE6hBZCfhzTg	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 13:04:31.369077+07	2026-03-09 13:04:31.369415+07	t
14462ea9-85ef-4bbd-bc09-28fb3f098a54	3f16bd25-2aff-42df-b193-bbc3faffb12b	VI1J6SOM8XUwyrNvUB79GQgL4tj5aFMgiepwpjq5mmEx8zpt8GcL4wZ78CM2rCu_UrAJb39lCfjwp4ugF273mg	\N	python-requests/2.32.5	f	2026-03-10 16:41:06.084692+07	2026-03-09 16:41:06.084826+07	t
fde16135-b8c1-4582-b2c5-cc6c709ced2a	c87443dc-b777-47b2-a1f0-e345c92e1b47	JJ8cG10v6Rd8m9sateosn3SWTnXa4OvBfRg57ZbFzbUBg7S7c9aT_GCDcLhiTeYKnVbSoupNCnoh284Mpg9C7w	\N	curl/8.7.1	f	2026-03-10 14:46:17.910841+07	2026-03-09 14:46:17.911512+07	f
612e6909-26c6-4a58-a3f2-3ba32097cb45	c87443dc-b777-47b2-a1f0-e345c92e1b47	dSSOxYarUNMp2-lxytAK5ROa9dV_gWUdBdCEPe1l1q9VdGT4SPnkocGwPL_7PBRlyMViqsvoI8NokVhAVrBaGw	\N	curl/8.7.1	f	2026-03-10 14:50:16.449512+07	2026-03-09 14:50:16.450137+07	t
9cecb1a0-491c-4375-8723-00968a7f38c1	0ee84d65-6576-4d63-b767-0359b06b8ec5	eTs7JUzgtzJDjopcSFWbfmWbuFIOSa_94JiZcj3q9wYxc-tn8mZ_ucgfRK5yle504NEcy2WWO_C11xmqBT0Wrg	\N	python-requests/2.32.5	f	2026-03-10 14:56:31.530997+07	2026-03-09 14:56:31.53129+07	f
a7c7d1e3-a607-4b5b-94c6-de72cb306448	c87443dc-b777-47b2-a1f0-e345c92e1b47	syuDIZA3mEmJ5KeJdhoazn-9GBxrLKSLhLNaDnkILumqD7ytnTSGTDw3Ms4vvuyRh00Z9-EX3Ns7MAdxYgI1sQ	\N	Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36	f	2026-03-10 15:06:45.151119+07	2026-03-09 15:06:45.151461+07	t
098701b8-ffa7-4492-9875-de2b4aaf6197	0ee84d65-6576-4d63-b767-0359b06b8ec5	JOT0bGEgr5Z6Fx7nUz4gGvg345Ym2Pcb1XUlEDHMR1KL7Il9iUSmnImloMhmaOApBc2N1TsrK5G-8yuCyHwThg	\N	python-requests/2.32.5	f	2026-03-10 15:46:49.692922+07	2026-03-09 15:46:49.693928+07	f
af58f899-e5fd-4f09-ae06-c1903bdc1d17	0ee84d65-6576-4d63-b767-0359b06b8ec5	QS2EvgXK6aOuDjClV5mwEQxaQ939xdcTUIHV2U4AUMw_RO_zscXCp2292F1IzSXIhredYwt8i50PvlWKYK8UaA	\N	python-requests/2.32.5	f	2026-03-10 15:48:27.318335+07	2026-03-09 15:48:27.318795+07	f
5e498c9f-78ad-4836-9e03-6683c9ab7bba	0ee84d65-6576-4d63-b767-0359b06b8ec5	v5ueWqRBH_mqh5WeR-_hJAfDiNbPhimBkOzYtGNvmlGoKNnAf0xVuYEx-z5K7HaCyvDm1q1p4HPkkNgBviWrJQ	\N	python-requests/2.32.5	f	2026-03-10 15:49:07.189541+07	2026-03-09 15:49:07.189702+07	f
fc169dfd-eda8-4a50-b67c-18d14d95f448	0ee84d65-6576-4d63-b767-0359b06b8ec5	_iVCRGXxdrCaxqdkXXKNTSAJwpJi_jI7REcTUU44m4b8R0W5W5-nbre9BaVB17BX6KUvuc92F_cKjaVnv_uMSw	\N	python-requests/2.32.5	f	2026-03-10 15:49:36.509801+07	2026-03-09 15:49:36.509982+07	f
3ca77136-78cd-4ed3-9d1b-37b0da1ea963	0ee84d65-6576-4d63-b767-0359b06b8ec5	YXhDZ_2wnBwFbRfwWSjJ-cdCuvZMYXRRzJp4OAHogwfakeH53y_w5QQ9jIBO6Z1rqpF9hQyzzBHOyJXoVHscAg	\N	python-requests/2.32.5	f	2026-03-10 15:50:16.83406+07	2026-03-09 15:50:16.834157+07	f
2dbc0d31-5dcc-4e47-a00c-c170d451a1ab	0ee84d65-6576-4d63-b767-0359b06b8ec5	YFHJK8PJg--y_HuuOmK46CfsVnzUN5J0Gp_fckvDq2PWnR4rLkKAIMjLNj71CD7S-Sq8DuTkQWx9QeuIksv3oQ	\N	python-requests/2.32.5	f	2026-03-10 15:53:32.112189+07	2026-03-09 15:53:32.112297+07	f
8c440037-a1c7-4e9b-8f7f-e5c529e527de	0ee84d65-6576-4d63-b767-0359b06b8ec5	t4AbBiSk2diIAHgcUb08-oam6XXoN1lxibzWibtfJ_wlJ0xceuFpa54nYqc8K2KNh_at1DwhiiGEqFCFZONsIA	\N	python-requests/2.32.5	f	2026-03-10 15:53:56.663134+07	2026-03-09 15:53:56.663297+07	f
765fe6c8-1d10-4b77-8e09-6c96026d3694	0ee84d65-6576-4d63-b767-0359b06b8ec5	jhEVrzPmDSX6l-OWj90f9qMr3zoiXixcYplCxc6RYDcMUl_vtPlOEKPSyC39kpLYU8lpe3iMuq3kZjzono0vYQ	\N	python-requests/2.32.5	f	2026-03-10 15:54:56.23963+07	2026-03-09 15:54:56.239981+07	f
b2be878c-3da8-44df-9e3c-fc529ad5724b	0ee84d65-6576-4d63-b767-0359b06b8ec5	_J-bgqHWMeZXWIunYkLvHJLDRh3vRygtjqtlyljl_RuIr5j7f7akqrftI4gmYEgHAljd6NUFzGtoJQjhUZAXnA	\N	python-requests/2.32.5	f	2026-03-10 16:15:21.573617+07	2026-03-09 16:15:21.57397+07	f
ce52ff90-edb0-4c93-8301-110855016aef	af18c46f-fd0c-4617-a959-5d7e2fb556b2	LGwAnTwj0Sq5dpKuAuLaD85WCzr3lUrEaQLoEXxISPMCgliD3TVsvT4i4D7THtDcmsVY9VqDD4ylXi_5VKT6aA	\N	python-requests/2.32.5	f	2026-03-10 16:40:20.517715+07	2026-03-09 16:40:20.518414+07	f
d405e64b-01c4-442c-bd80-980e26015c67	edc6dda5-e21d-420d-a088-003dbcb5a827	a8OSOlp7umZB886MYaNzr_Nv7Va57oEsfbpFUdZTlrnu4WCm440lfVPE_ZvD9QxACJWz7zB2DTu1RJHoghuGxw	\N	python-requests/2.32.5	f	2026-03-10 16:40:38.703715+07	2026-03-09 16:40:38.704109+07	f
44ef28c6-c068-4913-9a01-154a5a228260	edc6dda5-e21d-420d-a088-003dbcb5a827	msFuwlb-sQqcWm4I93OnHQfwqG4dKvQ_cCWQcUIepdFg-Jg4WmkrPIgCcmiTJzkpsTCtlGBJSjGh2H5yuWilvA	\N	python-requests/2.32.5	f	2026-03-10 16:40:38.718881+07	2026-03-09 16:40:38.719037+07	t
b33f9473-beec-4f7b-949e-0a95abc0db9a	2e5509db-67a5-46dc-87bb-391c6727a782	T6G9NjvFSqGJPDtx4h6aYVz6KXrBPO9aEJtYIrA2oeeHNcR1Eifj9ydAcYI0v1qSdkGFRI19NoS5yfq2TezcWQ	\N	python-requests/2.32.5	f	2026-03-10 16:42:05.857897+07	2026-03-09 16:42:05.858242+07	f
fb0af4c0-1a5b-4d7f-b6cc-12a6670f8797	2e5509db-67a5-46dc-87bb-391c6727a782	t2svLZi9vyIAXSKaglWulc4Mz8QQmKIwBxfFNpQ52S-GesEApyIxyFgbXHy2tC4Y6y-Ekv3R3qHOTVYO0G5SDw	\N	python-requests/2.32.5	f	2026-03-10 16:42:05.872449+07	2026-03-09 16:42:05.872619+07	t
d1683803-eae5-459e-8408-02ddd5b5c7ed	893aef07-c9e2-403d-9d2b-27eabcfcd78f	x9GrxkrrbEGOW_0z1Fv-1wKN309qiUbHXX2Y7PEB9i_BI3C70eVzGex6g14JldWklJ5QoGLqomYNY224XALPIQ	\N	python-requests/2.32.5	f	2026-03-10 16:42:05.893276+07	2026-03-09 16:42:05.893515+07	f
17af783b-5a6c-4a81-aea1-bb82dfdcd05e	893aef07-c9e2-403d-9d2b-27eabcfcd78f	zwq0loUmxoCTlIqyyUXSoBqb31FUCpOUZ0Qhd_aJEhNPeZVUuLBJyB86ohEPEZHZxOiQ6TWvFA2FduWiw4ed5w	\N	python-requests/2.32.5	f	2026-03-10 16:42:05.907458+07	2026-03-09 16:42:05.907617+07	t
458a9ae2-fb4c-4d05-be1f-6b371a35b076	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	Q146TWm9VcUAt_GJWILvvCPzeYL1j2ovV6j99zigibk_OhY6JS1e_kYjHYHkoqO3g8H09n_t3C2y8AIJ7WsVJA	\N	python-requests/2.32.5	f	2026-03-10 16:49:01.735597+07	2026-03-09 16:49:01.735881+07	f
c0ed60d0-bf02-4984-b237-dd1d77f93878	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	t3S2LeIMB7uiWyVpe9qD-SiV8ZWq-eI06D0FKCniSpn_80zYfNCeeOYJk2OlcIQRSsQs6qN1bXU02c5patmlmA	\N	python-requests/2.32.5	f	2026-03-10 16:49:01.752408+07	2026-03-09 16:49:01.752774+07	t
e3ce3f4d-0f45-4a65-8581-cba368777ff0	230101db-e689-47ad-9307-076f4f0e55f1	XU35d7V8oPXVED-lpayEH4KtD8a_cpTwg7lQx3m2XMqaYTgIjf9bVgyHvJoc0SZ4KdhVx06UBa_474GTXaj0Wg	\N	python-requests/2.32.5	f	2026-03-10 16:49:01.773401+07	2026-03-09 16:49:01.773481+07	f
a9b4e3ff-d3ad-46c8-81fb-dfa8496c89bf	230101db-e689-47ad-9307-076f4f0e55f1	Hf-LGKPRsf9eC5NE5G_t5P4OKFRqhDCH2Y0ezyCtfuO_GCq9h80vmJysxmOOlVQgAAxh5w37QR_g8Onn0qvpQA	\N	python-requests/2.32.5	f	2026-03-10 16:49:01.787185+07	2026-03-09 16:49:01.787308+07	t
\.


--
-- Data for Name: user_settings; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.user_settings (id, user_id, totp_secret, totp_enabled, language, email_notifications, push_notifications, updated_at, currency, timezone) FROM stdin;
536bec7c-debc-47f7-a460-dfe4ae6ab312	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	\N	f	en	t	t	2026-03-08 22:12:00.159174+07	USD	UTC
c4210691-69a1-4f1d-8969-bdeaaedf5f3c	f91c696d-f3e2-4525-80e1-01a29d700062	\N	f	en	t	t	2026-03-08 22:12:20.974173+07	USD	UTC
72dc46c2-7174-4468-9499-b5e17decc05a	db7174f5-c055-470c-ad50-b969ccfea5a4	\N	f	en	t	t	2026-03-08 22:12:45.787196+07	USD	UTC
15d4ef0c-30e4-482a-8407-deb5bbc527e0	dee2990f-8070-4d34-ab17-e7fb2c041b8b	\N	f	en	t	t	2026-03-09 01:08:00.598481+07	USD	UTC
026ebee3-867e-455a-853b-f4fc9495fbde	c87443dc-b777-47b2-a1f0-e345c92e1b47	OTWFD2QXKU5554XJO3KXHFL3BGXEOUH6	t	en	t	t	2026-03-09 11:53:33.052027+07	USD	UTC
5c3fc033-6450-4275-8382-b63534745015	0ee84d65-6576-4d63-b767-0359b06b8ec5	\N	f	en	t	t	2026-03-09 14:15:44.822507+07	USD	UTC
1a7b3c9f-8d58-47ea-a11d-fb2632eedb8b	b45f038b-3e75-41e7-806a-0db9ca5b9272	\N	f	en	t	t	2026-03-09 14:54:18.329653+07	USD	UTC
ac5d28cf-9e0e-47a4-bd9f-ca8a1e039010	af18c46f-fd0c-4617-a959-5d7e2fb556b2	\N	f	en	t	t	2026-03-09 16:40:20.510219+07	USD	UTC
32f04a9b-05f0-45b2-af47-1ef3974ffa28	edc6dda5-e21d-420d-a088-003dbcb5a827	\N	f	en	t	t	2026-03-09 16:40:38.695863+07	USD	UTC
6c0505e8-dcce-4ce3-85a1-3bc1a331c3a6	3f16bd25-2aff-42df-b193-bbc3faffb12b	\N	f	en	t	t	2026-03-09 16:41:06.062913+07	USD	UTC
65f5cdfc-f28b-475b-a895-7e522d84bbd4	2e5509db-67a5-46dc-87bb-391c6727a782	\N	f	en	t	t	2026-03-09 16:42:05.851866+07	USD	UTC
54998b52-2089-4f2a-8263-40e6b4199504	893aef07-c9e2-403d-9d2b-27eabcfcd78f	\N	f	en	t	t	2026-03-09 16:42:05.889963+07	USD	UTC
6d29c074-ad9a-4e23-83fa-e37d24c197cc	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	\N	f	en	t	t	2026-03-09 16:49:01.728884+07	USD	UTC
e06d5487-0cd2-4a6c-9e7b-f9f23522448f	230101db-e689-47ad-9307-076f4f0e55f1	\N	f	en	t	t	2026-03-09 16:49:01.770463+07	USD	UTC
\.


--
-- Data for Name: user_tiers; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.user_tiers (id, user_id, tier_id, invested_12m, updated_at) FROM stdin;
cf69866a-19fd-4f1c-b301-f51d5a73f5b4	0ee84d65-6576-4d63-b767-0359b06b8ec5	4	500000	2026-03-09 14:14:28.919601+07
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.users (id, email, password_hash, email_verified, avatar_url, status, created_at, updated_at, reset_password) FROM stdin;
edc6dda5-e21d-420d-a088-003dbcb5a827	user_97f814c8@poool.app	$argon2id$v=19$m=19456,t=2,p=1$h4Kv7ieMtFCu8z/tipCmEg$2tB44+dtS5aeMiIUWvDWHy8AhT4d3RPNJdhZ+TBs88w	f	\N	active	2026-03-09 16:40:38.695863+07	2026-03-09 16:40:38.695863+07	f
3f16bd25-2aff-42df-b193-bbc3faffb12b	user_20022459@poool.app	$argon2id$v=19$m=19456,t=2,p=1$MUxSPbx6x4ZL8yz2FkGntg$hYdrM+TvtWXPA74Cy7BkVHCNkRj50G5rHPGMwsKwZD4	f	\N	active	2026-03-09 16:41:06.062913+07	2026-03-09 16:41:06.062913+07	f
2e5509db-67a5-46dc-87bb-391c6727a782	dev1_1eeac5@poool.app	$argon2id$v=19$m=19456,t=2,p=1$Koej32dT05xJvPnrzLXm+g$tgOpyMddok5Gl8/PU0w3MbhHu6w7ddTX/rCg29Gmlbc	f	\N	active	2026-03-09 16:42:05.851866+07	2026-03-09 16:42:05.851866+07	f
893aef07-c9e2-403d-9d2b-27eabcfcd78f	dev2_9f8620@poool.app	$argon2id$v=19$m=19456,t=2,p=1$/MHmA0HEtu+SLkdXjf/yzw$zmxGseSk0GVOE7r2Aw4cvf+Q0/X1mpz6LHLgXlUCyiM	f	\N	active	2026-03-09 16:42:05.889963+07	2026-03-09 16:42:05.889963+07	f
c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	dev1_888d65@poool.app	$argon2id$v=19$m=19456,t=2,p=1$Ole2PJiGi1jRLc3VOcLWgQ$2eTDYbVNoV/J1ERbyPAXHpPVqk44DMoaYLcy1JC2a3I	f	\N	active	2026-03-09 16:49:01.728884+07	2026-03-09 16:49:01.728884+07	f
230101db-e689-47ad-9307-076f4f0e55f1	dev2_a3e6ed@poool.app	$argon2id$v=19$m=19456,t=2,p=1$0K5tkgYdj7MrxPittUIudw$3A8mgkmRY6qWUMdyJ4PamqEF+0/1XG1Mk1XAVTwwGIk	f	\N	active	2026-03-09 16:49:01.770463+07	2026-03-09 16:49:01.770463+07	f
0ee84d65-6576-4d63-b767-0359b06b8ec5	test@poool.app	$argon2id$v=19$m=65536,t=3,p=4$z+ZczJX3H43LOE0p8oeykA$4LR061AO9+asrAJ4DlBVr/GUuKqlEL4kCNjk4YsQJZ4	t	https://storage.googleapis.com/poool-assets-primary/avatars/0ee84d65-6576-4d63-b767-0359b06b8ec5/e9ef3862-983d-4d7f-898b-95c0234ff092.jpg	active	2026-03-08 20:12:09.351095+07	2026-03-09 23:23:44.662061+07	f
d0e276a2-46f7-48ec-a06c-6e778bfa46bc	referred_1772982719@example.com	$argon2id$v=19$m=65536,t=3,p=4$Q2rFiZCGvqM9mR/wgeYvfQ$gtDzqRJt9PEAUryfPs8i6Y+x1Ma2n2vZ2If69LdALtc	f	\N	active	2026-03-08 22:12:00.159174+07	2026-03-09 11:57:31.705017+07	f
f91c696d-f3e2-4525-80e1-01a29d700062	referred_1772982740@example.com	$argon2id$v=19$m=65536,t=3,p=4$Q2rFiZCGvqM9mR/wgeYvfQ$gtDzqRJt9PEAUryfPs8i6Y+x1Ma2n2vZ2If69LdALtc	f	\N	active	2026-03-08 22:12:20.974173+07	2026-03-09 11:57:31.705017+07	f
db7174f5-c055-470c-ad50-b969ccfea5a4	referred_1772982765@example.com	$argon2id$v=19$m=65536,t=3,p=4$Q2rFiZCGvqM9mR/wgeYvfQ$gtDzqRJt9PEAUryfPs8i6Y+x1Ma2n2vZ2If69LdALtc	f	\N	suspended	2026-03-08 22:12:45.787196+07	2026-03-09 11:57:31.705017+07	f
dee2990f-8070-4d34-ab17-e7fb2c041b8b	qa_test_final@poool.app	$argon2id$v=19$m=65536,t=3,p=4$Q2rFiZCGvqM9mR/wgeYvfQ$gtDzqRJt9PEAUryfPs8i6Y+x1Ma2n2vZ2If69LdALtc	f	\N	active	2026-03-09 01:08:00.598481+07	2026-03-09 11:57:31.705017+07	f
b45f038b-3e75-41e7-806a-0db9ca5b9272	jonas.freiwald@poool.app	$2a$12$VitNhndIpsYYv0bddQlsf.cbvuO/hDlHzoQLOYajSK7eknVV7Gukq	t	\N	active	2026-03-09 11:57:37.679511+07	2026-03-09 14:51:46.523374+07	f
c87443dc-b777-47b2-a1f0-e345c92e1b47	admin@poool.app	$argon2id$v=19$m=65536,t=3,p=4$JioHFLwzjJWop6zejkVMRA$pBqAnDA3uNY4OoscSpSWgrOlyIm/6JU6RV5nHgCcSz8	t	\N	active	2026-03-08 20:11:23.435279+07	2026-03-09 16:25:09.666951+07	f
af18c46f-fd0c-4617-a959-5d7e2fb556b2	test_debug2@poool.app	$argon2id$v=19$m=19456,t=2,p=1$ZyKnEQyVLhgi2Xo+m2i2WA$ZW+4AKCi0dC9uBevE0YGlaT0rBrpRhYzq34id/RH29Y	f	\N	active	2026-03-09 16:40:20.510219+07	2026-03-09 16:40:20.510219+07	f
\.


--
-- Data for Name: wallet_transactions; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.wallet_transactions (id, wallet_id, type, status, amount_cents, description, external_ref_id, related_order_id, metadata, created_at, completed_at, currency) FROM stdin;
c697ee7f-d77e-421e-881e-732885fc5ce4	cf149ba2-269d-4c0c-8980-21d96e5580b0	deposit	completed	5000	\N	\N	\N	\N	2026-03-09 16:33:47.673542+07	\N	USD
e5e075eb-83d9-4f08-8df0-5c204cfa5c89	cf149ba2-269d-4c0c-8980-21d96e5580b0	withdrawal	completed	-5000	\N	\N	\N	\N	2026-03-09 16:33:47.694545+07	\N	USD
7a5580f4-daaf-435c-b569-ed3b4dbdf0c0	cf149ba2-269d-4c0c-8980-21d96e5580b0	purchase	completed	-26680	Asset purchase	\N	3d19040e-c2c0-4fdb-908d-29e3d0e4ac23	\N	2026-03-09 18:15:17.378169+07	\N	USD
62f0037f-8266-482f-93e3-0baa62c62a55	cf149ba2-269d-4c0c-8980-21d96e5580b0	purchase	completed	-26680	Asset purchase	\N	e992d372-fec0-4041-a495-6d7b732415e3	\N	2026-03-09 19:28:31.785911+07	\N	USD
a1e5cf56-3478-4fd8-830f-af4b4f6ef064	cf149ba2-269d-4c0c-8980-21d96e5580b0	purchase	completed	-26680	Asset purchase	\N	264fdd24-94e6-4261-9e53-b22d57d78ea6	\N	2026-03-09 19:30:02.542928+07	\N	USD
82e3e80a-cfcb-4263-b715-53155a2babf4	cf149ba2-269d-4c0c-8980-21d96e5580b0	purchase	completed	-373520	Asset purchase	\N	144ba81d-a451-4190-8bbf-9f0bc0a93a02	\N	2026-03-09 21:45:14.723443+07	\N	USD
06472088-4997-42f5-b0a5-0485b74ff38e	7d6ffb03-3721-47d0-a0c6-edfe2bd530b8	purchase	completed	-198000	Asset purchase	\N	5ff8f3cb-44a9-45da-a3ca-6e675945f212	\N	2026-03-09 21:46:48.206756+07	\N	USD
\.


--
-- Data for Name: wallets; Type: TABLE DATA; Schema: public; Owner: martin
--

COPY public.wallets (id, user_id, wallet_type, balance_cents, created_at, updated_at, currency) FROM stdin;
b18a6209-e33d-4b0e-b505-913414b468ce	c87443dc-b777-47b2-a1f0-e345c92e1b47	rewards	0	2026-03-08 21:34:36.395448+07	2026-03-08 21:34:36.395448+07	USD
dbd4621e-c04d-486b-a626-3cf05a2bf2a3	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	cash	0	2026-03-08 22:12:00.159174+07	2026-03-08 22:12:00.159174+07	USD
9eb8d582-9a10-46b2-8399-06e8f04d8eca	d0e276a2-46f7-48ec-a06c-6e778bfa46bc	rewards	0	2026-03-08 22:12:00.159174+07	2026-03-08 22:12:00.159174+07	USD
cc12418f-97b0-485f-8ee9-71c8173c45f6	f91c696d-f3e2-4525-80e1-01a29d700062	cash	0	2026-03-08 22:12:20.974173+07	2026-03-08 22:12:20.974173+07	USD
69bba32c-af8d-48ce-b42e-7b2cd8d3e8fb	f91c696d-f3e2-4525-80e1-01a29d700062	rewards	0	2026-03-08 22:12:20.974173+07	2026-03-08 22:12:20.974173+07	USD
38822591-fb87-401e-8eec-c061096cf7f6	db7174f5-c055-470c-ad50-b969ccfea5a4	cash	0	2026-03-08 22:12:45.787196+07	2026-03-08 22:12:45.787196+07	USD
754672f2-99e0-4497-8753-c50db917545d	db7174f5-c055-470c-ad50-b969ccfea5a4	rewards	0	2026-03-08 22:12:45.787196+07	2026-03-08 22:12:45.787196+07	USD
eea3474f-29a6-42e7-840e-176c46c0130c	dee2990f-8070-4d34-ab17-e7fb2c041b8b	cash	0	2026-03-09 01:08:00.598481+07	2026-03-09 01:08:00.598481+07	USD
39d51739-205f-4308-9e3f-2b79537f831a	dee2990f-8070-4d34-ab17-e7fb2c041b8b	rewards	0	2026-03-09 01:08:00.598481+07	2026-03-09 01:08:00.598481+07	USD
a33bd712-1500-4294-8da8-a3e0e6cd8d1a	0ee84d65-6576-4d63-b767-0359b06b8ec5	rewards	50000000	2026-03-08 20:12:16.034361+07	2026-03-09 19:30:02.505047+07	USD
cf149ba2-269d-4c0c-8980-21d96e5580b0	0ee84d65-6576-4d63-b767-0359b06b8ec5	cash	49599800	2026-03-08 20:12:16.034361+07	2026-03-09 21:45:14.723443+07	USD
7d6ffb03-3721-47d0-a0c6-edfe2bd530b8	c87443dc-b777-47b2-a1f0-e345c92e1b47	cash	499802000	2026-03-08 21:34:36.395448+07	2026-03-09 21:46:48.206756+07	USD
fc805fac-ecfe-419e-8e6f-3609c020dbcc	b45f038b-3e75-41e7-806a-0db9ca5b9272	cash	0	2026-03-09 11:57:37.679511+07	2026-03-09 11:57:37.679511+07	USD
0fb154c9-7bbf-44f9-9445-ef12a4e8458b	b45f038b-3e75-41e7-806a-0db9ca5b9272	rewards	0	2026-03-09 11:57:37.679511+07	2026-03-09 11:57:37.679511+07	USD
14b6812c-f654-4cdb-a62f-4ab46fa5f878	af18c46f-fd0c-4617-a959-5d7e2fb556b2	cash	0	2026-03-09 16:40:20.510219+07	2026-03-09 16:40:20.510219+07	USD
f0fb8c5e-5a81-45c3-b699-2915f8baff81	af18c46f-fd0c-4617-a959-5d7e2fb556b2	rewards	0	2026-03-09 16:40:20.510219+07	2026-03-09 16:40:20.510219+07	USD
449f41f9-6961-4aef-84df-64487a38fa8c	edc6dda5-e21d-420d-a088-003dbcb5a827	cash	0	2026-03-09 16:40:38.695863+07	2026-03-09 16:40:38.695863+07	USD
4557b8de-19da-49a0-97d4-efdc196202b6	edc6dda5-e21d-420d-a088-003dbcb5a827	rewards	0	2026-03-09 16:40:38.695863+07	2026-03-09 16:40:38.695863+07	USD
6948c471-5bb4-42df-a030-eb419e8c0101	3f16bd25-2aff-42df-b193-bbc3faffb12b	cash	0	2026-03-09 16:41:06.062913+07	2026-03-09 16:41:06.062913+07	USD
215c16ec-33cd-4e7e-a64d-fead63f7ca67	3f16bd25-2aff-42df-b193-bbc3faffb12b	rewards	0	2026-03-09 16:41:06.062913+07	2026-03-09 16:41:06.062913+07	USD
b6df164c-13d0-40ff-b9a9-c46b96e8fe63	2e5509db-67a5-46dc-87bb-391c6727a782	cash	0	2026-03-09 16:42:05.851866+07	2026-03-09 16:42:05.851866+07	USD
62b3b042-5dd5-4f16-8022-d4bcd8b7c4d9	2e5509db-67a5-46dc-87bb-391c6727a782	rewards	0	2026-03-09 16:42:05.851866+07	2026-03-09 16:42:05.851866+07	USD
ebfc6a2f-2b47-4596-8557-3353b35c3cdd	893aef07-c9e2-403d-9d2b-27eabcfcd78f	cash	0	2026-03-09 16:42:05.889963+07	2026-03-09 16:42:05.889963+07	USD
3866f936-15f7-4ca4-8c0f-854ad3d953ca	893aef07-c9e2-403d-9d2b-27eabcfcd78f	rewards	0	2026-03-09 16:42:05.889963+07	2026-03-09 16:42:05.889963+07	USD
550fc55e-5165-4875-897d-4df700a4cbf2	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	cash	0	2026-03-09 16:49:01.728884+07	2026-03-09 16:49:01.728884+07	USD
ad1f6d53-2e5e-498e-af07-77b08f2aaf53	c5c58c0f-0441-4692-b9f1-f8c25db2e1a1	rewards	0	2026-03-09 16:49:01.728884+07	2026-03-09 16:49:01.728884+07	USD
edb14e7a-6941-472e-bcc4-1fca0ed3a64c	230101db-e689-47ad-9307-076f4f0e55f1	cash	0	2026-03-09 16:49:01.770463+07	2026-03-09 16:49:01.770463+07	USD
b13a266f-f2b9-496c-a8a2-67918bc64f7c	230101db-e689-47ad-9307-076f4f0e55f1	rewards	0	2026-03-09 16:49:01.770463+07	2026-03-09 16:49:01.770463+07	USD
\.


--
-- Name: audit_logs_id_seq; Type: SEQUENCE SET; Schema: public; Owner: martin
--

SELECT pg_catalog.setval('public.audit_logs_id_seq', 197, true);


--
-- Name: email_logs_id_seq; Type: SEQUENCE SET; Schema: public; Owner: martin
--

SELECT pg_catalog.setval('public.email_logs_id_seq', 3, true);


--
-- Name: invoice_number_seq; Type: SEQUENCE SET; Schema: public; Owner: martin
--

SELECT pg_catalog.setval('public.invoice_number_seq', 49, true);


--
-- Name: tiers_id_seq; Type: SEQUENCE SET; Schema: public; Owner: martin
--

SELECT pg_catalog.setval('public.tiers_id_seq', 13, true);


--
-- Name: admin_approval_requests admin_approval_requests_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_approval_requests
    ADD CONSTRAINT admin_approval_requests_pkey PRIMARY KEY (id);


--
-- Name: admin_invitations admin_invitations_email_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_invitations
    ADD CONSTRAINT admin_invitations_email_key UNIQUE (email);


--
-- Name: admin_invitations admin_invitations_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_invitations
    ADD CONSTRAINT admin_invitations_pkey PRIMARY KEY (id);


--
-- Name: admin_invitations admin_invitations_token_hash_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_invitations
    ADD CONSTRAINT admin_invitations_token_hash_key UNIQUE (token_hash);


--
-- Name: admin_permissions admin_permissions_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_permissions
    ADD CONSTRAINT admin_permissions_pkey PRIMARY KEY (id);


--
-- Name: admin_permissions admin_permissions_role_id_permission_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_permissions
    ADD CONSTRAINT admin_permissions_role_id_permission_key UNIQUE (role_id, permission);


--
-- Name: asset_documents asset_documents_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_documents
    ADD CONSTRAINT asset_documents_pkey PRIMARY KEY (id);


--
-- Name: asset_financials asset_financials_asset_id_period_month_period_year_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_financials
    ADD CONSTRAINT asset_financials_asset_id_period_month_period_year_key UNIQUE (asset_id, period_month, period_year);


--
-- Name: asset_financials asset_financials_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_financials
    ADD CONSTRAINT asset_financials_pkey PRIMARY KEY (id);


--
-- Name: asset_images asset_images_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_images
    ADD CONSTRAINT asset_images_pkey PRIMARY KEY (id);


--
-- Name: asset_milestones asset_milestones_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_milestones
    ADD CONSTRAINT asset_milestones_pkey PRIMARY KEY (id);


--
-- Name: assets assets_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.assets
    ADD CONSTRAINT assets_pkey PRIMARY KEY (id);


--
-- Name: assets assets_slug_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.assets
    ADD CONSTRAINT assets_slug_key UNIQUE (slug);


--
-- Name: audit_logs audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_pkey PRIMARY KEY (id);


--
-- Name: cart_items cart_items_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.cart_items
    ADD CONSTRAINT cart_items_pkey PRIMARY KEY (id);


--
-- Name: cart_items cart_items_user_id_asset_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.cart_items
    ADD CONSTRAINT cart_items_user_id_asset_id_key UNIQUE (user_id, asset_id);


--
-- Name: deposit_requests deposit_requests_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.deposit_requests
    ADD CONSTRAINT deposit_requests_pkey PRIMARY KEY (id);


--
-- Name: developer_projects developer_projects_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.developer_projects
    ADD CONSTRAINT developer_projects_pkey PRIMARY KEY (id);


--
-- Name: dividend_payouts dividend_payouts_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.dividend_payouts
    ADD CONSTRAINT dividend_payouts_pkey PRIMARY KEY (id);


--
-- Name: email_logs email_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.email_logs
    ADD CONSTRAINT email_logs_pkey PRIMARY KEY (id);


--
-- Name: email_templates email_templates_name_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.email_templates
    ADD CONSTRAINT email_templates_name_key UNIQUE (name);


--
-- Name: email_templates email_templates_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.email_templates
    ADD CONSTRAINT email_templates_pkey PRIMARY KEY (id);


--
-- Name: investment_limits investment_limits_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.investment_limits
    ADD CONSTRAINT investment_limits_pkey PRIMARY KEY (id);


--
-- Name: investment_limits investment_limits_user_id_limit_year_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.investment_limits
    ADD CONSTRAINT investment_limits_user_id_limit_year_key UNIQUE (user_id, limit_year);


--
-- Name: investments investments_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.investments
    ADD CONSTRAINT investments_pkey PRIMARY KEY (id);


--
-- Name: investments investments_user_id_asset_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.investments
    ADD CONSTRAINT investments_user_id_asset_id_key UNIQUE (user_id, asset_id);


--
-- Name: invoices invoices_invoice_number_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_invoice_number_key UNIQUE (invoice_number);


--
-- Name: invoices invoices_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_pkey PRIMARY KEY (id);


--
-- Name: kyc_documents kyc_documents_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.kyc_documents
    ADD CONSTRAINT kyc_documents_pkey PRIMARY KEY (id);


--
-- Name: kyc_records kyc_records_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.kyc_records
    ADD CONSTRAINT kyc_records_pkey PRIMARY KEY (id);


--
-- Name: notifications notifications_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.notifications
    ADD CONSTRAINT notifications_pkey PRIMARY KEY (id);


--
-- Name: oauth_accounts oauth_accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.oauth_accounts
    ADD CONSTRAINT oauth_accounts_pkey PRIMARY KEY (id);


--
-- Name: oauth_accounts oauth_accounts_provider_provider_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.oauth_accounts
    ADD CONSTRAINT oauth_accounts_provider_provider_id_key UNIQUE (provider, provider_id);


--
-- Name: order_items order_items_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.order_items
    ADD CONSTRAINT order_items_pkey PRIMARY KEY (id);


--
-- Name: orders orders_order_number_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.orders
    ADD CONSTRAINT orders_order_number_key UNIQUE (order_number);


--
-- Name: orders orders_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.orders
    ADD CONSTRAINT orders_pkey PRIMARY KEY (id);


--
-- Name: password_reset_tokens password_reset_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_pkey PRIMARY KEY (id);


--
-- Name: password_reset_tokens password_reset_tokens_token_hash_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_token_hash_key UNIQUE (token_hash);


--
-- Name: payment_methods payment_methods_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.payment_methods
    ADD CONSTRAINT payment_methods_pkey PRIMARY KEY (id);


--
-- Name: platform_settings platform_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.platform_settings
    ADD CONSTRAINT platform_settings_pkey PRIMARY KEY (key);


--
-- Name: referral_clicks referral_clicks_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_clicks
    ADD CONSTRAINT referral_clicks_pkey PRIMARY KEY (id);


--
-- Name: referral_codes referral_codes_code_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_codes
    ADD CONSTRAINT referral_codes_code_key UNIQUE (code);


--
-- Name: referral_codes referral_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_codes
    ADD CONSTRAINT referral_codes_pkey PRIMARY KEY (id);


--
-- Name: referral_codes referral_codes_user_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_codes
    ADD CONSTRAINT referral_codes_user_id_key UNIQUE (user_id);


--
-- Name: referral_tracking referral_tracking_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_tracking
    ADD CONSTRAINT referral_tracking_pkey PRIMARY KEY (id);


--
-- Name: referral_tracking referral_tracking_referred_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_tracking
    ADD CONSTRAINT referral_tracking_referred_id_key UNIQUE (referred_id);


--
-- Name: rewards_balances rewards_balances_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.rewards_balances
    ADD CONSTRAINT rewards_balances_pkey PRIMARY KEY (id);


--
-- Name: rewards_balances rewards_balances_user_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.rewards_balances
    ADD CONSTRAINT rewards_balances_user_id_key UNIQUE (user_id);


--
-- Name: roles roles_name_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_name_key UNIQUE (name);


--
-- Name: roles roles_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_pkey PRIMARY KEY (id);


--
-- Name: support_ticket_attachments support_ticket_attachments_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_ticket_attachments
    ADD CONSTRAINT support_ticket_attachments_pkey PRIMARY KEY (id);


--
-- Name: support_ticket_replies support_ticket_replies_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_ticket_replies
    ADD CONSTRAINT support_ticket_replies_pkey PRIMARY KEY (id);


--
-- Name: support_tickets support_tickets_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_tickets
    ADD CONSTRAINT support_tickets_pkey PRIMARY KEY (id);


--
-- Name: tiers tiers_name_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.tiers
    ADD CONSTRAINT tiers_name_key UNIQUE (name);


--
-- Name: tiers tiers_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.tiers
    ADD CONSTRAINT tiers_pkey PRIMARY KEY (id);


--
-- Name: user_consents user_consents_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_consents
    ADD CONSTRAINT user_consents_pkey PRIMARY KEY (id);


--
-- Name: user_profiles user_profiles_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_profiles
    ADD CONSTRAINT user_profiles_pkey PRIMARY KEY (id);


--
-- Name: user_profiles user_profiles_user_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_profiles
    ADD CONSTRAINT user_profiles_user_id_key UNIQUE (user_id);


--
-- Name: user_roles user_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_pkey PRIMARY KEY (id);


--
-- Name: user_roles user_roles_user_id_role_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_user_id_role_id_key UNIQUE (user_id, role_id);


--
-- Name: user_sessions user_sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_sessions
    ADD CONSTRAINT user_sessions_pkey PRIMARY KEY (id);


--
-- Name: user_sessions user_sessions_session_token_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_sessions
    ADD CONSTRAINT user_sessions_session_token_key UNIQUE (session_token);


--
-- Name: user_settings user_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_settings
    ADD CONSTRAINT user_settings_pkey PRIMARY KEY (id);


--
-- Name: user_settings user_settings_user_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_settings
    ADD CONSTRAINT user_settings_user_id_key UNIQUE (user_id);


--
-- Name: user_tiers user_tiers_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_tiers
    ADD CONSTRAINT user_tiers_pkey PRIMARY KEY (id);


--
-- Name: user_tiers user_tiers_user_id_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_tiers
    ADD CONSTRAINT user_tiers_user_id_key UNIQUE (user_id);


--
-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: wallet_transactions wallet_transactions_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.wallet_transactions
    ADD CONSTRAINT wallet_transactions_pkey PRIMARY KEY (id);


--
-- Name: wallets wallets_pkey; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.wallets
    ADD CONSTRAINT wallets_pkey PRIMARY KEY (id);


--
-- Name: wallets wallets_user_id_wallet_type_currency_key; Type: CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.wallets
    ADD CONSTRAINT wallets_user_id_wallet_type_currency_key UNIQUE (user_id, wallet_type, currency);


--
-- Name: idx_admin_inv_token; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_admin_inv_token ON public.admin_invitations USING btree (token_hash);


--
-- Name: idx_approval_requester; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_approval_requester ON public.admin_approval_requests USING btree (requester_id);


--
-- Name: idx_approval_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_approval_status ON public.admin_approval_requests USING btree (status);


--
-- Name: idx_asset_docs_asset; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_asset_docs_asset ON public.asset_documents USING btree (asset_id);


--
-- Name: idx_asset_fin_asset; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_asset_fin_asset ON public.asset_financials USING btree (asset_id);


--
-- Name: idx_asset_images_asset; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_asset_images_asset ON public.asset_images USING btree (asset_id);


--
-- Name: idx_asset_milestones_asset; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_asset_milestones_asset ON public.asset_milestones USING btree (asset_id);


--
-- Name: idx_assets_developer; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_assets_developer ON public.assets USING btree (developer_user_id);


--
-- Name: idx_assets_slug; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_assets_slug ON public.assets USING btree (slug);


--
-- Name: idx_assets_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_assets_status ON public.assets USING btree (funding_status);


--
-- Name: idx_assets_type; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_assets_type ON public.assets USING btree (asset_type);


--
-- Name: idx_audit_action; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_audit_action ON public.audit_logs USING btree (action);


--
-- Name: idx_audit_created; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_audit_created ON public.audit_logs USING btree (created_at DESC);


--
-- Name: idx_audit_entity; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_audit_entity ON public.audit_logs USING btree (entity_type, entity_id);


--
-- Name: idx_audit_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_audit_user ON public.audit_logs USING btree (actor_user_id);


--
-- Name: idx_cart_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_cart_user ON public.cart_items USING btree (user_id);


--
-- Name: idx_deposit_req_provider_ref; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_deposit_req_provider_ref ON public.deposit_requests USING btree (provider_reference);


--
-- Name: idx_deposit_req_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_deposit_req_status ON public.deposit_requests USING btree (status);


--
-- Name: idx_deposit_req_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_deposit_req_user ON public.deposit_requests USING btree (user_id);


--
-- Name: idx_dev_projects_developer; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_dev_projects_developer ON public.developer_projects USING btree (developer_id);


--
-- Name: idx_dividends_investment; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_dividends_investment ON public.dividend_payouts USING btree (investment_id);


--
-- Name: idx_dividends_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_dividends_user ON public.dividend_payouts USING btree (user_id);


--
-- Name: idx_email_logs_sent_at; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_email_logs_sent_at ON public.email_logs USING btree (sent_at DESC);


--
-- Name: idx_email_logs_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_email_logs_status ON public.email_logs USING btree (status);


--
-- Name: idx_email_logs_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_email_logs_user ON public.email_logs USING btree (user_id);


--
-- Name: idx_inv_limits_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_inv_limits_user ON public.investment_limits USING btree (user_id);


--
-- Name: idx_investments_asset; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_investments_asset ON public.investments USING btree (asset_id);


--
-- Name: idx_investments_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_investments_user ON public.investments USING btree (user_id);


--
-- Name: idx_invoices_number; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_invoices_number ON public.invoices USING btree (invoice_number);


--
-- Name: idx_invoices_order; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_invoices_order ON public.invoices USING btree (order_id);


--
-- Name: idx_invoices_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_invoices_user ON public.invoices USING btree (user_id);


--
-- Name: idx_kyc_docs_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_kyc_docs_status ON public.kyc_documents USING btree (status);


--
-- Name: idx_kyc_docs_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_kyc_docs_user ON public.kyc_documents USING btree (user_id);


--
-- Name: idx_kyc_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_kyc_status ON public.kyc_records USING btree (status);


--
-- Name: idx_kyc_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_kyc_user ON public.kyc_records USING btree (user_id);


--
-- Name: idx_notifications_unread; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_notifications_unread ON public.notifications USING btree (user_id, is_read) WHERE (is_read = false);


--
-- Name: idx_notifications_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_notifications_user ON public.notifications USING btree (user_id);


--
-- Name: idx_oauth_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_oauth_user ON public.oauth_accounts USING btree (user_id);


--
-- Name: idx_order_items_order; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_order_items_order ON public.order_items USING btree (order_id);


--
-- Name: idx_orders_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_orders_status ON public.orders USING btree (status);


--
-- Name: idx_orders_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_orders_user ON public.orders USING btree (user_id);


--
-- Name: idx_payment_methods_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_payment_methods_user ON public.payment_methods USING btree (user_id);


--
-- Name: idx_prt_token; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_prt_token ON public.password_reset_tokens USING btree (token_hash);


--
-- Name: idx_referral_clicks_code; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_referral_clicks_code ON public.referral_clicks USING btree (code);


--
-- Name: idx_referral_clicks_subid; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_referral_clicks_subid ON public.referral_clicks USING btree (subid);


--
-- Name: idx_referral_tracking_subid; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_referral_tracking_subid ON public.referral_tracking USING btree (subid);


--
-- Name: idx_sessions_token; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_sessions_token ON public.user_sessions USING btree (session_token);


--
-- Name: idx_sessions_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_sessions_user ON public.user_sessions USING btree (user_id);


--
-- Name: idx_support_attachments_reply; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_support_attachments_reply ON public.support_ticket_attachments USING btree (reply_id);


--
-- Name: idx_support_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_support_status ON public.support_tickets USING btree (status);


--
-- Name: idx_support_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_support_user ON public.support_tickets USING btree (user_id);


--
-- Name: idx_ticket_replies_author; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_ticket_replies_author ON public.support_ticket_replies USING btree (author_id);


--
-- Name: idx_ticket_replies_ticket; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_ticket_replies_ticket ON public.support_ticket_replies USING btree (ticket_id);


--
-- Name: idx_user_consents_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_user_consents_user ON public.user_consents USING btree (user_id);


--
-- Name: idx_user_consents_version; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_user_consents_version ON public.user_consents USING btree (terms_version);


--
-- Name: idx_user_roles_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_user_roles_user ON public.user_roles USING btree (user_id);


--
-- Name: idx_users_email; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_users_email ON public.users USING btree (email);


--
-- Name: idx_wallets_user; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_wallets_user ON public.wallets USING btree (user_id);


--
-- Name: idx_wtx_created; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_wtx_created ON public.wallet_transactions USING btree (created_at DESC);


--
-- Name: idx_wtx_status; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_wtx_status ON public.wallet_transactions USING btree (status);


--
-- Name: idx_wtx_type; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_wtx_type ON public.wallet_transactions USING btree (type);


--
-- Name: idx_wtx_wallet; Type: INDEX; Schema: public; Owner: martin
--

CREATE INDEX idx_wtx_wallet ON public.wallet_transactions USING btree (wallet_id);


--
-- Name: admin_approval_requests set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.admin_approval_requests FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: assets set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.assets FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: cart_items set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.cart_items FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: deposit_requests set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.deposit_requests FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: developer_projects set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.developer_projects FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: email_templates set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.email_templates FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: investments set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.investments FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: kyc_records set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.kyc_records FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: support_tickets set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.support_tickets FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: user_profiles set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.user_profiles FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: user_settings set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.user_settings FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: users set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: wallets set_updated_at; Type: TRIGGER; Schema: public; Owner: martin
--

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.wallets FOR EACH ROW EXECUTE FUNCTION public.trigger_set_updated_at();


--
-- Name: admin_approval_requests admin_approval_requests_approver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_approval_requests
    ADD CONSTRAINT admin_approval_requests_approver_id_fkey FOREIGN KEY (approver_id) REFERENCES public.users(id);


--
-- Name: admin_approval_requests admin_approval_requests_requester_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_approval_requests
    ADD CONSTRAINT admin_approval_requests_requester_id_fkey FOREIGN KEY (requester_id) REFERENCES public.users(id);


--
-- Name: admin_invitations admin_invitations_invited_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_invitations
    ADD CONSTRAINT admin_invitations_invited_by_fkey FOREIGN KEY (invited_by) REFERENCES public.users(id);


--
-- Name: admin_invitations admin_invitations_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_invitations
    ADD CONSTRAINT admin_invitations_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.roles(id);


--
-- Name: admin_permissions admin_permissions_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.admin_permissions
    ADD CONSTRAINT admin_permissions_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.roles(id) ON DELETE CASCADE;


--
-- Name: asset_documents asset_documents_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_documents
    ADD CONSTRAINT asset_documents_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id) ON DELETE CASCADE;


--
-- Name: asset_financials asset_financials_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_financials
    ADD CONSTRAINT asset_financials_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id) ON DELETE CASCADE;


--
-- Name: asset_images asset_images_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_images
    ADD CONSTRAINT asset_images_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id) ON DELETE CASCADE;


--
-- Name: asset_milestones asset_milestones_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.asset_milestones
    ADD CONSTRAINT asset_milestones_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id) ON DELETE CASCADE;


--
-- Name: assets assets_developer_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.assets
    ADD CONSTRAINT assets_developer_user_id_fkey FOREIGN KEY (developer_user_id) REFERENCES public.users(id);


--
-- Name: audit_logs audit_logs_actor_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_actor_user_id_fkey FOREIGN KEY (actor_user_id) REFERENCES public.users(id);


--
-- Name: cart_items cart_items_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.cart_items
    ADD CONSTRAINT cart_items_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id);


--
-- Name: cart_items cart_items_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.cart_items
    ADD CONSTRAINT cart_items_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: deposit_requests deposit_requests_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.deposit_requests
    ADD CONSTRAINT deposit_requests_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: developer_projects developer_projects_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.developer_projects
    ADD CONSTRAINT developer_projects_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id);


--
-- Name: developer_projects developer_projects_developer_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.developer_projects
    ADD CONSTRAINT developer_projects_developer_id_fkey FOREIGN KEY (developer_id) REFERENCES public.users(id);


--
-- Name: dividend_payouts dividend_payouts_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.dividend_payouts
    ADD CONSTRAINT dividend_payouts_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id);


--
-- Name: dividend_payouts dividend_payouts_investment_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.dividend_payouts
    ADD CONSTRAINT dividend_payouts_investment_id_fkey FOREIGN KEY (investment_id) REFERENCES public.investments(id);


--
-- Name: dividend_payouts dividend_payouts_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.dividend_payouts
    ADD CONSTRAINT dividend_payouts_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: dividend_payouts dividend_payouts_wallet_tx_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.dividend_payouts
    ADD CONSTRAINT dividend_payouts_wallet_tx_id_fkey FOREIGN KEY (wallet_tx_id) REFERENCES public.wallet_transactions(id);


--
-- Name: email_logs email_logs_template_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.email_logs
    ADD CONSTRAINT email_logs_template_id_fkey FOREIGN KEY (template_id) REFERENCES public.email_templates(id);


--
-- Name: email_logs email_logs_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.email_logs
    ADD CONSTRAINT email_logs_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: investment_limits investment_limits_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.investment_limits
    ADD CONSTRAINT investment_limits_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: investments investments_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.investments
    ADD CONSTRAINT investments_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id);


--
-- Name: investments investments_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.investments
    ADD CONSTRAINT investments_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: invoices invoices_order_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_order_id_fkey FOREIGN KEY (order_id) REFERENCES public.orders(id);


--
-- Name: invoices invoices_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: kyc_documents kyc_documents_kyc_record_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.kyc_documents
    ADD CONSTRAINT kyc_documents_kyc_record_id_fkey FOREIGN KEY (kyc_record_id) REFERENCES public.kyc_records(id) ON DELETE SET NULL;


--
-- Name: kyc_documents kyc_documents_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.kyc_documents
    ADD CONSTRAINT kyc_documents_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: kyc_records kyc_records_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.kyc_records
    ADD CONSTRAINT kyc_records_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: notifications notifications_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.notifications
    ADD CONSTRAINT notifications_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: oauth_accounts oauth_accounts_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.oauth_accounts
    ADD CONSTRAINT oauth_accounts_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: order_items order_items_asset_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.order_items
    ADD CONSTRAINT order_items_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES public.assets(id);


--
-- Name: order_items order_items_order_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.order_items
    ADD CONSTRAINT order_items_order_id_fkey FOREIGN KEY (order_id) REFERENCES public.orders(id) ON DELETE CASCADE;


--
-- Name: orders orders_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.orders
    ADD CONSTRAINT orders_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: password_reset_tokens password_reset_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: payment_methods payment_methods_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.payment_methods
    ADD CONSTRAINT payment_methods_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: platform_settings platform_settings_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.platform_settings
    ADD CONSTRAINT platform_settings_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id);


--
-- Name: referral_clicks referral_clicks_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_clicks
    ADD CONSTRAINT referral_clicks_code_fkey FOREIGN KEY (code) REFERENCES public.referral_codes(code) ON DELETE CASCADE;


--
-- Name: referral_codes referral_codes_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_codes
    ADD CONSTRAINT referral_codes_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: referral_tracking referral_tracking_referred_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_tracking
    ADD CONSTRAINT referral_tracking_referred_id_fkey FOREIGN KEY (referred_id) REFERENCES public.users(id);


--
-- Name: referral_tracking referral_tracking_referrer_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.referral_tracking
    ADD CONSTRAINT referral_tracking_referrer_id_fkey FOREIGN KEY (referrer_id) REFERENCES public.users(id);


--
-- Name: rewards_balances rewards_balances_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.rewards_balances
    ADD CONSTRAINT rewards_balances_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: support_ticket_attachments support_ticket_attachments_reply_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_ticket_attachments
    ADD CONSTRAINT support_ticket_attachments_reply_id_fkey FOREIGN KEY (reply_id) REFERENCES public.support_ticket_replies(id) ON DELETE CASCADE;


--
-- Name: support_ticket_replies support_ticket_replies_author_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_ticket_replies
    ADD CONSTRAINT support_ticket_replies_author_id_fkey FOREIGN KEY (author_id) REFERENCES public.users(id);


--
-- Name: support_ticket_replies support_ticket_replies_ticket_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_ticket_replies
    ADD CONSTRAINT support_ticket_replies_ticket_id_fkey FOREIGN KEY (ticket_id) REFERENCES public.support_tickets(id) ON DELETE CASCADE;


--
-- Name: support_tickets support_tickets_assigned_to_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_tickets
    ADD CONSTRAINT support_tickets_assigned_to_fkey FOREIGN KEY (assigned_to) REFERENCES public.users(id);


--
-- Name: support_tickets support_tickets_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.support_tickets
    ADD CONSTRAINT support_tickets_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: user_consents user_consents_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_consents
    ADD CONSTRAINT user_consents_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_profiles user_profiles_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_profiles
    ADD CONSTRAINT user_profiles_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_roles user_roles_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.roles(id);


--
-- Name: user_roles user_roles_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_sessions user_sessions_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_sessions
    ADD CONSTRAINT user_sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_settings user_settings_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_settings
    ADD CONSTRAINT user_settings_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_tiers user_tiers_tier_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_tiers
    ADD CONSTRAINT user_tiers_tier_id_fkey FOREIGN KEY (tier_id) REFERENCES public.tiers(id);


--
-- Name: user_tiers user_tiers_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.user_tiers
    ADD CONSTRAINT user_tiers_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: wallet_transactions wallet_transactions_wallet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.wallet_transactions
    ADD CONSTRAINT wallet_transactions_wallet_id_fkey FOREIGN KEY (wallet_id) REFERENCES public.wallets(id);


--
-- Name: wallets wallets_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: martin
--

ALTER TABLE ONLY public.wallets
    ADD CONSTRAINT wallets_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict 9dWWtz6hPYWzJmkp0Zo15A3JRVAwhMlncUUbGjn5sz04zeTiFygYEGEoBsoi7Tx

