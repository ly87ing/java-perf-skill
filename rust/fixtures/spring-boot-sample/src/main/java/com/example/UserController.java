package com.example;

import org.springframework.web.bind.annotation.*;
import org.springframework.beans.factory.annotation.Autowired;
import java.util.List;

/**
 * 示例 Controller - 包含典型性能问题
 */
@RestController
@RequestMapping("/api/users")
public class UserController {

    @Autowired  // P1: 字段注入
    private UserService userService;

    @GetMapping
    public List<User> getUsers() {
        // N+1 问题：Service 内循环调用 Repository
        return userService.findAllWithDetails();
    }
    
    @GetMapping("/{id}")
    public User getUser(@PathVariable Long id) {
        return userService.findById(id).block();  // P0: 阻塞调用
    }
}
