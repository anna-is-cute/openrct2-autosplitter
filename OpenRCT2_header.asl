// OpenRCT2 Autosplitter by anna
// https://git.anna.lgbt/anna/openrct2-autosplitter

state("openrct2", "v0.4.1 (be518f4)") {
    byte gScreenFlags : 0xefc597;
    ulong gScenarioCompletedCompanyValue : 0xefdc50;
}

state("openrct2", "v0.4.0 (c6302a1)") {
    byte gScreenFlags : 0xdecee8;
    ulong gScenarioCompletedCompanyValue : 0xdee5a0;
}

state("openrct2", "v0.3.5.1 (61c67af)") {
    byte gScreenFlags : 0xb5d903;
    ulong gScenarioCompletedCompanyValue : 0xb5f5d8;
}

state("openrct2", "v0.3.5 (b9bc8d0)") {
    byte gScreenFlags : 0xb5fff3;
    ulong gScenarioCompletedCompanyValue : 0xb61cc8;
}

state("openrct2", "v0.3.4.1 (5087e77)") {
    byte gScreenFlags : 0xba8d54;
    ulong gScenarioCompletedCompanyValue : 0xbaaa20;
}

state("openrct2", "v0.3.4 (e0daac9)") {
    byte gScreenFlags : 0xba8d44;
    ulong gScenarioCompletedCompanyValue : 0xbaaa10;
}

state("openrct2", "v0.3.3 (3f65f28)") {
    byte gScreenFlags : 0xb97c84;
    ulong gScenarioCompletedCompanyValue : 0xb9a7bc;
}

state("openrct2", "v0.3.2 (cea5fab)") {
    byte gScreenFlags : 0xb045a2;
    ulong gScenarioCompletedCompanyValue : 0xb06c6c;
}

state("openrct2", "v0.3.1 (d01dcea)") {
    byte gScreenFlags : 0xadd163;
    ulong gScenarioCompletedCompanyValue : 0xae078c;
}

state("openrct2", "v0.3.0 (135cc10)") {
    byte gScreenFlags : 0xa61773;
    ulong gScenarioCompletedCompanyValue : 0xa63f3c;
}

state("openrct2", "v0.2.6 (6c3c857)") {
    byte gScreenFlags : 0x910373;
    ulong gScenarioCompletedCompanyValue : 0xcb68bc;
}

state("openrct2", "v0.2.5 (4f6e77e)") {
    byte gScreenFlags : 0x90a363;
    ulong gScenarioCompletedCompanyValue : 0xcb06a0;
}

state("openrct2", "v0.2.4 (d645338)") {
    byte gScreenFlags : "openrct2.dll", 0xa669ef;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xe0da24;
}

state("openrct2", "v0.2.3 (ac7a1eb)") {
    byte gScreenFlags : "openrct2.dll", 0xa51ff6;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xc79128;
}

state("openrct2", "v0.2.2 (298c9f5)") {
    byte gScreenFlags : "openrct2.dll", 0xa0f63f;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xcefdb4;
}

state("openrct2", "v0.2.1 (8ac731e)") {
    byte gScreenFlags : "openrct2.dll", 0x9d676a;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xcb6204;
}

state("openrct2", "v0.2.0 (0aff800)") {
    byte gScreenFlags : "openrct2.dll", 0x9c2eae;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xc92568;
}

state("openrct2", "v0.1.2 (0e7c0f7)") {
    byte gScreenFlags : "openrct2.dll", 0x9eb556;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xca1e98;
}

state("openrct2", "v0.1.1 (4601265)") {
    byte gScreenFlags : "openrct2.dll", 0x8dfc53;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xf3ba64;
}

state("openrct2", "v0.1.0 (6d1f732)") {
    byte gScreenFlags : "openrct2.dll", 0xf4cdc2;
    ulong gScenarioCompletedCompanyValue : "openrct2.dll", 0xf4c524;
}

init {
    var module = modules.First();
    string hash = vars.CalcModuleHash(module);
    switch (hash) {
        case "e4b01b45a22535995e6521857ea77bf71d0c4c5fcca2d72e8d242fdf3ea77b37":
            version = "v0.4.1 (be518f4)";
            break;
        case "800e53b5505f97ec465c1592945a3f76db7f451658276b415adb250f6e6a2a8f":
            version = "v0.4.0 (c6302a1)";
            break;
        case "87c62541f2f4c5d42f5415155ce8fac84d43565a6da860afe046f265cb738f0d":
            version = "v0.3.5.1 (61c67af)";
            break;
        case "a8a80bd6de6675e112dc27d74ffbe359b3f9746000a1e7a484eae5f1424281b5":
            version = "v0.3.5 (b9bc8d0)";
            break;
        case "19c08aca7b3b12160db173a481a72a3acf727d2cc842ece10fe5f5a59fabc8d9":
            version = "v0.3.4.1 (5087e77)";
            break;
        case "7c4fde5f3f6a01ea0f291b921b6260b1a73bb90731b57d5a9606ac940446780c":
            version = "v0.3.4 (e0daac9)";
            break;
        case "96f1a3cadeaf1e36ae38b0f91085d468be26b026948c4e96fbd599cba6c6b45d":
            version = "v0.3.3 (3f65f28)";
            break;
        case "c3908f3291d5e007a9e719ead410669e1fe577759b2d5eed48e75d56c677f740":
            version = "v0.3.2 (cea5fab)";
            break;
        case "e75d4d8f3455ba359cb0266c0ef3b49ed926c27a6ac7457bbdb1c020ccd8aabb":
            version = "v0.3.1 (d01dcea)";
            break;
        case "960d49fa00c658886e66ec119f07668584ab768b21b10beade5d9d6a164c2a29":
            version = "v0.3.0 (135cc10)";
            break;
        case "1fba35105b12c0292b16d9f003897aa5a5884f3412346da68919325827c92c0d":
            version = "v0.2.6 (6c3c857)";
            break;
        case "f90120d4d1bb5595b23a71ea0bd52bc22d5105ca28201b5f47d14beab4cb9309":
            version = "v0.2.5 (4f6e77e)";
            break;
        case "4d798b22ba0455cb1fc27565e5ffcb3912eeb655f2c5f5d41e61b6e4abdebfea":
            version = "v0.2.4 (d645338)";
            break;
        case "20ebff6e0e6f4b8c7b9e93f7d9465e63a674c1581f78e1cee6745aac4d09c153":
            version = "v0.2.3 (ac7a1eb)";
            break;
        case "7d0818603af935e960af602805b07a57a9235549b48880a51e858ef54f2676a7":
            version = "v0.2.2 (298c9f5)";
            break;
        case "8811e80831182297410ef94b7e5e45f302753865609dc00439e8c0c8c42d2297":
            version = "v0.2.1 (8ac731e)";
            break;
        case "71eba971aeee310c03106ba67f61769f01817168d3d4a68f2069dd04e75f9901":
            version = "v0.2.0 (0aff800)";
            break;
        case "acd53a0c6d5759c93773eb8988ddb9657e9a54489329d7d16679aa8d541c715d":
            version = "v0.1.2 (0e7c0f7)";
            break;
        case "16d3acefa040aa0c40c79a63e88f180a541ad2d8b778ca1cb6f4161138706b0a":
            version = "v0.1.1 (4601265)";
            break;
        case "ca49efaf5aca4b57def8662131e3e2b65f7b3772e2787c24ebed266ecd323fcb":
            version = "v0.1.0 (6d1f732)";
            break;
    }
}

