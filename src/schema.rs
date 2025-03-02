use diesel::table;

table! {
    courses (id) {
        id -> Integer,
        title -> Text,
        description -> Text,
        languages -> Text,
        programminglanguages -> Text,
        gamificationruleconditions -> Text,
        gamificationcomplexrules -> Text,
        gamificationruleresults -> Text,
        createdat -> Date,
        updatedat -> Date,
    }
}

table! {
    games (id) {
        id -> Integer,
        title -> Text,
        public -> Bool,
        active -> Bool,
        description -> Text,
        course -> Integer,
        programminglanguage -> Text,
        modulelock -> Float,
        exerciselock -> Bool,
        totalexercises -> Integer,
        startdate -> Date,
        enddate -> Date,
        createdat -> Date,
        updatedat -> Date,
    }
}

table! {
    modules (id) {
        id -> Integer,
        course -> Integer,
        order -> Integer,
        title -> Text,
        description -> Text,
        language -> Text,
        startdate -> Date,
        enddate -> Date,
    }
}

table! {
    exercises (id) {
        id -> Integer,
        version -> Integer,
        module -> Integer,
        order -> Integer,
        title -> Text,
        description -> Text,
        language -> Text,
        programminglanguage -> Text,
        initcode -> Text,
        precode -> Text,
        postcode -> Text,
        testcode -> Text,
        checksource -> Text,
        hidden -> Bool,
        locked -> Bool,
        mode -> Text,
        modeparameters -> Text,
        difficulty -> Text,
        createdat -> Date,
        updatedat -> Date,
    }
}

table! {
    submissions (id) {
        id -> Integer,
        exercise -> Integer,
        player -> Integer,
        client -> Text,
        submittedcode -> Text,
        metrics -> Text,
        result -> Double,
        resultdescription -> Text,
        feedback -> Text,
        earnedrewards -> Text,
        enteredat -> Date,
        submittedat -> Date,
    }
}

table! {
    players (id) {
        id -> Integer,
        email -> Text,
        displayname -> Text,
        displayavatar -> Text,
        points -> Integer,
        createdat -> Date,
        lastactive -> Date,
    }
}

table! {
    groups (id) {
        id -> Integer,
        displayname -> Text,
        displayavatar -> Text,
    }
}

table! {
    playergroups (player, group) {
        player -> Integer,
        group -> Integer,
        joinedat -> Date,
        leftat -> Nullable<Date>,
    }
}

table! {
    playerregistrations (id) {
        id -> Integer,
        player -> Integer,
        game -> Integer,
        language -> Text,
        progress -> Integer,
        gamestate -> Text,
        savedat -> Date,
        joinedat -> Date,
        leftat -> Nullable<Date>,
    }
}

table! {
    playerunlocks (player, exercise) {
        player -> Integer,
        exercise -> Integer,
        unlockedat -> Date,
    }
}

table! {
    rewards (id) {
        id -> Integer,
        course -> Integer,
        name -> Text,
        description -> Text,
        messagewhenwon -> Text,
        imageurl -> Text,
        validperiod -> Interval
    }
}

table! {
    playerrewards (player, reward, game) {
        player -> Integer,
        reward -> Integer,
        game -> Nullable<Integer>,
        count -> Integer,
        usedcount -> Integer,
        obtainedat -> Date,
        expiresat -> Date,
    }
}
