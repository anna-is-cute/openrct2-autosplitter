startup {
    Func<ProcessModuleWow64Safe, string> CalcModuleHash = (module) => {
        var exeHashBytes = new byte[0];
        using (var sha = System.Security.Cryptography.SHA256.Create()) {
            using (var s = File.Open(module.FileName, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)) {
                exeHashBytes = sha.ComputeHash(s);
            }
        }
        return string.Join("", exeHashBytes.Select(x => x.ToString("x2")));
    };

    vars.CalcModuleHash = CalcModuleHash;
}

// check source for screenflags info. 0 is playing, non-zero is something else
start {
    if (current.gScreenFlags == 0 && old.gScreenFlags > 0) {
        return true;
    }
}

reset {
    if (current.gScreenFlags > 0 && old.gScreenFlags == 0) {
        return true;
    }
}

// search for MONEY32_UNDEFINED or MONEY64_UNDEFINED
split {
    var isComplete = (current.gScenarioCompletedCompanyValue & 0xFFFFFFFF) != 0x80000000 && current.gScenarioCompletedCompanyValue != 0x8000000000000000;
    var isFailed = (current.gScenarioCompletedCompanyValue & 0xFFFFFFFF) == 0x80000001 || current.gScenarioCompletedCompanyValue == 0x8000000000000001;
    var wasIncomplete = (old.gScenarioCompletedCompanyValue & 0xFFFFFFFF) == 0x80000000 || old.gScenarioCompletedCompanyValue == 0x8000000000000000;
    if (current.gScreenFlags == 0 && isComplete && !isFailed && wasIncomplete) {
        return true;
    }
}
