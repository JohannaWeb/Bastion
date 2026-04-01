package app.juntos.alpha.api;

import app.juntos.alpha.auth.DidResolver;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

@RestController
@RequestMapping("/api/did")
@RequiredArgsConstructor
public class DidController {

    private final DidResolver didResolver;

    @GetMapping("/resolve")
    public Map<String, Object> resolve(@RequestParam String did) {
        return didResolver.resolve(did);
    }
}
