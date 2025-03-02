--
-- PostgreSQL database dump
--

-- Dumped from database version 17.2
-- Dumped by pg_dump version 17.2

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: courses; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.courses (
    id integer NOT NULL,
    title text NOT NULL,
    description text NOT NULL,
    languages text NOT NULL,
    programminglanguages text NOT NULL,
    gamificationruleconditions text NOT NULL,
    gamificationcomplexrules text NOT NULL,
    gamificationruleresults text NOT NULL,
    createdat date NOT NULL,
    updatedat date NOT NULL
);


ALTER TABLE public.courses OWNER TO postgres;

--
-- Name: course_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.course_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.course_id_seq OWNER TO postgres;

--
-- Name: course_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.course_id_seq OWNED BY public.courses.id;


--
-- Name: exercises; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.exercises (
    id integer NOT NULL,
    version integer NOT NULL,
    module integer NOT NULL,
    "order" integer NOT NULL,
    title character varying NOT NULL,
    description text NOT NULL,
    language character varying NOT NULL,
    programminglanguage character varying NOT NULL,
    initcode text NOT NULL,
    precode text NOT NULL,
    postcode text NOT NULL,
    testcode text NOT NULL,
    checksource text NOT NULL,
    hidden boolean NOT NULL,
    locked boolean NOT NULL,
    mode character varying NOT NULL,
    modeparameters text NOT NULL,
    difficulty character varying NOT NULL,
    createdat date NOT NULL,
    updatedat date NOT NULL
);


ALTER TABLE public.exercises OWNER TO postgres;

--
-- Name: exercises_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.exercises_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.exercises_id_seq OWNER TO postgres;

--
-- Name: exercises_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.exercises_id_seq OWNED BY public.exercises.id;


--
-- Name: games; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.games (
    id integer NOT NULL,
    title character varying NOT NULL,
    public boolean NOT NULL,
    active boolean NOT NULL,
    description text NOT NULL,
    course integer NOT NULL,
    programminglanguage character varying NOT NULL,
    modulelock real NOT NULL,
    exerciselock boolean NOT NULL,
    totalexercises integer NOT NULL,
    startdate date NOT NULL,
    enddate date NOT NULL,
    createdat date NOT NULL,
    updatedat date NOT NULL
);


ALTER TABLE public.games OWNER TO postgres;

--
-- Name: games_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.games_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.games_id_seq OWNER TO postgres;

--
-- Name: games_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.games_id_seq OWNED BY public.games.id;


--
-- Name: groups; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.groups (
    id integer NOT NULL,
    displayname character varying NOT NULL,
    displayavatar character varying NOT NULL
);


ALTER TABLE public.groups OWNER TO postgres;

--
-- Name: groups_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.groups_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.groups_id_seq OWNER TO postgres;

--
-- Name: groups_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.groups_id_seq OWNED BY public.groups.id;


--
-- Name: modules; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.modules (
    id integer NOT NULL,
    course integer NOT NULL,
    "order" integer NOT NULL,
    title character varying NOT NULL,
    description text NOT NULL,
    language character varying NOT NULL,
    startdate date NOT NULL,
    enddate date NOT NULL
);


ALTER TABLE public.modules OWNER TO postgres;

--
-- Name: modules_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.modules_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.modules_id_seq OWNER TO postgres;

--
-- Name: modules_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.modules_id_seq OWNED BY public.modules.id;


--
-- Name: playergroups; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.playergroups (
    player integer NOT NULL,
    "group" integer NOT NULL,
    joinedat date NOT NULL,
    leftat date
);


ALTER TABLE public.playergroups OWNER TO postgres;

--
-- Name: playerregistrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.playerregistrations (
    id integer NOT NULL,
    player integer NOT NULL,
    game integer NOT NULL,
    language character varying NOT NULL,
    progress integer NOT NULL,
    gamestate character varying NOT NULL,
    savedat date NOT NULL,
    joinedat date NOT NULL,
    leftat date
);


ALTER TABLE public.playerregistrations OWNER TO postgres;

--
-- Name: playerregistrations_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.playerregistrations_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.playerregistrations_id_seq OWNER TO postgres;

--
-- Name: playerregistrations_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.playerregistrations_id_seq OWNED BY public.playerregistrations.id;


--
-- Name: playerrewards; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.playerrewards (
    player integer NOT NULL,
    reward integer NOT NULL,
    game integer,
    count integer NOT NULL,
    usedcount integer NOT NULL,
    obtainedat date NOT NULL,
    expiresat date NOT NULL
);


ALTER TABLE public.playerrewards OWNER TO postgres;

--
-- Name: players; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.players (
    id integer NOT NULL,
    email character varying NOT NULL,
    displayname character varying NOT NULL,
    displayavatar character varying NOT NULL,
    points integer NOT NULL,
    createdat date NOT NULL,
    lastactive date NOT NULL
);


ALTER TABLE public.players OWNER TO postgres;

--
-- Name: players_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.players_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.players_id_seq OWNER TO postgres;

--
-- Name: players_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.players_id_seq OWNED BY public.players.id;


--
-- Name: playerunlocks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.playerunlocks (
    player integer NOT NULL,
    exercise integer NOT NULL,
    unlockedat date NOT NULL
);


ALTER TABLE public.playerunlocks OWNER TO postgres;

--
-- Name: rewards; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.rewards (
    id integer NOT NULL,
    course integer NOT NULL,
    name character varying NOT NULL,
    description text NOT NULL,
    messagewhenwon text NOT NULL,
    imageurl text NOT NULL,
    validperiod interval DEFAULT '1 mon'::interval NOT NULL
);


ALTER TABLE public.rewards OWNER TO postgres;

--
-- Name: rewards_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.rewards_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.rewards_id_seq OWNER TO postgres;

--
-- Name: rewards_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.rewards_id_seq OWNED BY public.rewards.id;


--
-- Name: submissions; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.submissions (
    id integer NOT NULL,
    exercise integer NOT NULL,
    player integer NOT NULL,
    client character varying NOT NULL,
    submittedcode text NOT NULL,
    metrics text NOT NULL,
    result double precision DEFAULT 100 NOT NULL,
    resultdescription text NOT NULL,
    feedback text NOT NULL,
    earnedrewards text NOT NULL,
    enteredat date NOT NULL,
    submittedat date NOT NULL
);


ALTER TABLE public.submissions OWNER TO postgres;

--
-- Name: submissions_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.submissions_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.submissions_id_seq OWNER TO postgres;

--
-- Name: submissions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.submissions_id_seq OWNED BY public.submissions.id;


--
-- Name: courses id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.courses ALTER COLUMN id SET DEFAULT nextval('public.course_id_seq'::regclass);


--
-- Name: exercises id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.exercises ALTER COLUMN id SET DEFAULT nextval('public.exercises_id_seq'::regclass);


--
-- Name: games id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.games ALTER COLUMN id SET DEFAULT nextval('public.games_id_seq'::regclass);


--
-- Name: groups id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups ALTER COLUMN id SET DEFAULT nextval('public.groups_id_seq'::regclass);


--
-- Name: modules id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.modules ALTER COLUMN id SET DEFAULT nextval('public.modules_id_seq'::regclass);


--
-- Name: playerregistrations id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerregistrations ALTER COLUMN id SET DEFAULT nextval('public.playerregistrations_id_seq'::regclass);


--
-- Name: players id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.players ALTER COLUMN id SET DEFAULT nextval('public.players_id_seq'::regclass);


--
-- Name: rewards id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.rewards ALTER COLUMN id SET DEFAULT nextval('public.rewards_id_seq'::regclass);


--
-- Name: submissions id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.submissions ALTER COLUMN id SET DEFAULT nextval('public.submissions_id_seq'::regclass);


--
-- Data for Name: courses; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.courses (id, title, description, languages, programminglanguages, gamificationruleconditions, gamificationcomplexrules, gamificationruleresults, createdat, updatedat) FROM stdin;
3	Course 3	Description of Course 3	Chinese, Japanese	Go, Kotlin	name: player 1 did 1 with 1 in 1 of 1 on 2000.01.01..2100.01.01 at 00:00..23:59 achieving 100		message	2024-12-06	2024-12-06
2	Course 2	Description of Course 2	French, German	C++, Ruby	name: player 1 did 1 with 1 in 1 of 1 on 2000.01.01..2100.01.01 at 00:00..23:59 achieving 100		coupon	2024-12-06	2024-12-06
1	Course 1	Description of Course 1	English, Spanish	Python, Java	name: player 1 did 1 with 1 in 1 of 1 on 2000.01.01..2100.01.01 at 00:00..23:59 achieving 100		badge	2024-12-06	2024-12-06
\.


--
-- Data for Name: exercises; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.exercises (id, version, module, "order", title, description, language, programminglanguage, initcode, precode, postcode, testcode, checksource, hidden, locked, mode, modeparameters, difficulty, createdat, updatedat) FROM stdin;
1	1	1	1	Exercise 1	Description of Exercise 1	English	Python	print("Hello")					f	f	Standard	{}	Easy	2024-12-06	2024-12-06
2	2	2	2	Exercise 2	Description of Exercise 2	Spanish	Java						t	f	Timed	{}	Medium	2024-12-06	2024-12-06
3	3	3	3	Exercise 3	Description of Exercise 3	French	C++						f	t	Challenge	{}	Hard	2024-12-06	2024-12-06
\.


--
-- Data for Name: games; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.games (id, title, public, active, description, course, programminglanguage, modulelock, exerciselock, totalexercises, startdate, enddate, createdat, updatedat) FROM stdin;
1	Game 1	t	t	Description of Game 1	1	Python	1.5	f	10	2024-01-01	2024-06-01	2024-12-06	2024-12-06
2	Game 2	f	t	Description of Game 2	2	Java	2	t	15	2024-02-01	2024-07-01	2024-12-06	2024-12-06
3	Game 3	t	f	Description of Game 3	3	C++	1	f	20	2024-03-01	2024-08-01	2024-12-06	2024-12-06
\.


--
-- Data for Name: groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.groups (id, displayname, displayavatar) FROM stdin;
1	Group 1	group1.png
2	Group 2	group2.png
3	Group 3	group3.png
\.


--
-- Data for Name: modules; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.modules (id, course, "order", title, description, language, startdate, enddate) FROM stdin;
1	1	1	Module 1	Description of Module 1	English	2024-01-10	2024-02-10
2	2	2	Module 2	Description of Module 2	Spanish	2024-02-15	2024-03-15
3	3	3	Module 3	Description of Module 3	French	2024-03-20	2024-04-20
\.


--
-- Data for Name: playergroups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.playergroups (player, "group", joinedat, leftat) FROM stdin;
1	1	2024-12-06	\N
2	2	2024-12-06	\N
3	3	2024-12-06	\N
\.


--
-- Data for Name: playerregistrations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.playerregistrations (id, player, game, language, progress, gamestate, savedat, joinedat, leftat) FROM stdin;
2	2	2	Spanish	75	InProgress	2024-12-06	2024-12-06	\N
3	3	3	French	100	Completed	2024-12-06	2024-12-06	\N
5	1	1	english	1		2024-12-08	2024-12-08	\N
1	1	1	Spanish	51		2024-12-06	2024-12-06	\N
4	1	1	english	2		2024-12-08	2024-12-08	\N
\.


--
-- Data for Name: playerrewards; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.playerrewards (player, reward, game, count, usedcount, obtainedat, expiresat) FROM stdin;
1	1	1	1	0	2024-12-06	2024-12-31
2	2	2	1	0	2024-12-06	2024-12-31
3	3	\N	2	1	2024-12-06	2024-12-31
1	1	1	1	0	2025-03-02	2025-04-01
1	2	1	1	0	2025-03-02	2025-04-01
\.


--
-- Data for Name: players; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.players (id, email, displayname, displayavatar, points, createdat, lastactive) FROM stdin;
1	player1@example.com	Player 1	avatar1.png	100	2024-12-06	2024-12-06
2	player2@example.com	Player 2	avatar2.png	200	2024-12-06	2024-12-06
3	player3@example.com	Player 3	avatar3.png	300	2024-12-06	2024-12-06
\.


--
-- Data for Name: playerunlocks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.playerunlocks (player, exercise, unlockedat) FROM stdin;
1	1	2025-03-02
1	1	2025-03-02
\.


--
-- Data for Name: rewards; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.rewards (id, course, name, description, messagewhenwon, imageurl, validperiod) FROM stdin;
3	3	Reward 3	Description of Reward 3	You won Reward 3!	reward3.png	1 mon
1	1	badge1	Description of Reward 1	You won Reward 1!	reward1.png	1 mon
2	2	badge2	Description of Reward 2	You won Reward 2!	reward2.png	1 mon
\.


--
-- Data for Name: submissions; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.submissions (id, exercise, player, client, submittedcode, metrics, result, resultdescription, feedback, earnedrewards, enteredat, submittedat) FROM stdin;
3	3	3	Desktop Client 1	cout << "Bonjour!";	{"time": "0.5s", "memory": "3MB"}	90	Success	Well done!	badge1,badge2	2024-12-06	2024-12-06
2	1	2	Mobile Client 1	System.out.println("Hola!");	{"time": "2s", "memory": "10MB"}	80	Partial Success	Consider optimizing your code.	badge1,badge2	2024-12-06	2024-12-06
\.


--
-- Name: course_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.course_id_seq', 3, true);


--
-- Name: exercises_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.exercises_id_seq', 3, true);


--
-- Name: games_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.games_id_seq', 3, true);


--
-- Name: groups_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.groups_id_seq', 3, true);


--
-- Name: modules_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.modules_id_seq', 3, true);


--
-- Name: playerregistrations_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.playerregistrations_id_seq', 6, true);


--
-- Name: players_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.players_id_seq', 3, true);


--
-- Name: rewards_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.rewards_id_seq', 3, true);


--
-- Name: submissions_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.submissions_id_seq', 10, true);


--
-- Name: courses course_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.courses
    ADD CONSTRAINT course_pkey PRIMARY KEY (id);


--
-- Name: exercises exercises_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.exercises
    ADD CONSTRAINT exercises_pkey PRIMARY KEY (id);


--
-- Name: games games_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.games
    ADD CONSTRAINT games_pkey PRIMARY KEY (id);


--
-- Name: groups groups_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups
    ADD CONSTRAINT groups_pkey PRIMARY KEY (id);


--
-- Name: modules modules_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.modules
    ADD CONSTRAINT modules_pkey PRIMARY KEY (id);


--
-- Name: playerregistrations playerregistrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerregistrations
    ADD CONSTRAINT playerregistrations_pkey PRIMARY KEY (id);


--
-- Name: players players_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_pkey PRIMARY KEY (id);


--
-- Name: rewards rewards_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.rewards
    ADD CONSTRAINT rewards_pkey PRIMARY KEY (id);


--
-- Name: submissions submissions_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.submissions
    ADD CONSTRAINT submissions_pkey PRIMARY KEY (id);


--
-- Name: exercises exercises_module_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.exercises
    ADD CONSTRAINT exercises_module_fkey FOREIGN KEY (module) REFERENCES public.modules(id);


--
-- Name: games games_course_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.games
    ADD CONSTRAINT games_course_fkey FOREIGN KEY (course) REFERENCES public.courses(id);


--
-- Name: modules modules_course_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.modules
    ADD CONSTRAINT modules_course_fkey FOREIGN KEY (course) REFERENCES public.courses(id);


--
-- Name: playergroups playergroups_group_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playergroups
    ADD CONSTRAINT playergroups_group_fkey FOREIGN KEY ("group") REFERENCES public.groups(id);


--
-- Name: playergroups playergroups_player_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playergroups
    ADD CONSTRAINT playergroups_player_fkey FOREIGN KEY (player) REFERENCES public.players(id);


--
-- Name: playerregistrations playerregistrations_game_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerregistrations
    ADD CONSTRAINT playerregistrations_game_fkey FOREIGN KEY (game) REFERENCES public.games(id);


--
-- Name: playerregistrations playerregistrations_player_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerregistrations
    ADD CONSTRAINT playerregistrations_player_fkey FOREIGN KEY (player) REFERENCES public.players(id);


--
-- Name: playerrewards playerrewards_game_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerrewards
    ADD CONSTRAINT playerrewards_game_fkey FOREIGN KEY (game) REFERENCES public.games(id);


--
-- Name: playerrewards playerrewards_player_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerrewards
    ADD CONSTRAINT playerrewards_player_fkey FOREIGN KEY (player) REFERENCES public.players(id);


--
-- Name: playerrewards playerrewards_reward_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerrewards
    ADD CONSTRAINT playerrewards_reward_fkey FOREIGN KEY (reward) REFERENCES public.rewards(id);


--
-- Name: playerunlocks playerunlocks_exercise_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerunlocks
    ADD CONSTRAINT playerunlocks_exercise_fkey FOREIGN KEY (exercise) REFERENCES public.exercises(id);


--
-- Name: playerunlocks playerunlocks_player_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.playerunlocks
    ADD CONSTRAINT playerunlocks_player_fkey FOREIGN KEY (player) REFERENCES public.players(id);


--
-- Name: rewards rewards_course_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.rewards
    ADD CONSTRAINT rewards_course_fkey FOREIGN KEY (course) REFERENCES public.courses(id);


--
-- Name: submissions submissions_exercise_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.submissions
    ADD CONSTRAINT submissions_exercise_fkey FOREIGN KEY (exercise) REFERENCES public.exercises(id);


--
-- Name: submissions submissions_player_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.submissions
    ADD CONSTRAINT submissions_player_fkey FOREIGN KEY (player) REFERENCES public.players(id);


--
-- PostgreSQL database dump complete
--

